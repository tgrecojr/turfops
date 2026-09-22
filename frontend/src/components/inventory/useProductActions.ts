import { useState } from "react";
import {
	deleteProduct,
	linkProductApplications,
	refreshProductProfile,
	updateProduct,
} from "../../api/client";
import type { LoggedProductSuggestion, Product } from "../../types/inventory";
import { factsOf } from "../../types/inventory";
import type { Suggestion } from "./ProductRow";

export type Busy = "status" | "refresh" | "archive" | "delete" | "link";

type Setter<T> = React.Dispatch<React.SetStateAction<T>>;

interface Deps {
	setProducts: Setter<Product[]>;
	setError: (msg: string | null) => void;
	setNotice: (msg: string | null) => void;
	setSuggestions: Setter<Map<number, Suggestion>>;
	setExpanded: Setter<Set<number>>;
	setLogVersion: Setter<number>;
	setPrefill: (s: LoggedProductSuggestion | null) => void;
	setShowForm: (open: boolean) => void;
}

/** The Inventory page's per-product actions: one in flight at a time, errors to the banner. */
export function useProductActions(deps: Deps) {
	const {
		setProducts,
		setError,
		setNotice,
		setSuggestions,
		setExpanded,
		setLogVersion,
		setPrefill,
		setShowForm,
	} = deps;
	const [busy, setBusy] = useState<{ id: number; what: Busy } | null>(null);

	const replace = (updated: Product) =>
		setProducts((prev) => prev.map((p) => (p.id === updated.id ? updated : p)));

	const run = async (id: number, what: Busy, fn: () => Promise<void>) => {
		setBusy({ id, what });
		try {
			await fn();
			setError(null);
		} catch (e) {
			setError(e instanceof Error ? e.message : "Request failed");
		} finally {
			setBusy(null);
		}
	};

	const save = (p: Product, patch: Partial<Product>) => {
		if (p.id == null) return Promise.resolve();
		const merged = { ...p, ...patch };
		return updateProduct(p.id, {
			...factsOf(merged),
			name: merged.name,
			brand: merged.brand,
			stock_status: merged.stock_status,
			notes: merged.notes,
			archived: merged.archived,
		}).then(replace);
	};

	const handleRefresh = (p: Product) =>
		run(p.id ?? -1, "refresh", async () => {
			if (p.id == null) return;
			const res = await refreshProductProfile(p.id);
			replace(res.product);
			setSuggestions((prev) => {
				const next = new Map(prev);
				if (res.changed_fields.length > 0 && p.id != null)
					next.set(p.id, {
						facts: res.suggested_facts,
						changed: res.changed_fields,
					});
				else if (p.id != null) next.delete(p.id);
				return next;
			});
			setExpanded((prev) => (p.id == null ? prev : new Set(prev).add(p.id)));
		});

	const handleDelete = (p: Product) => {
		if (p.id == null || !confirm(`Delete "${p.name}" from your inventory?`))
			return;
		const id = p.id;
		run(id, "delete", async () => {
			await deleteProduct(id);
			setProducts((prev) => prev.filter((x) => x.id !== id));
		});
	};

	const handleLink = (p: Product) =>
		run(p.id ?? -1, "link", async () => {
			if (p.id == null) return;
			const { linked } = await linkProductApplications(p.id);
			setNotice(
				linked === 0
					? `No unlinked applications named "${p.name}" were found.`
					: `Linked ${linked} past application${linked === 1 ? "" : "s"} to ${p.name}.`,
			);
			setLogVersion((v) => v + 1);
		});

	const handleAddFromLog = (s: LoggedProductSuggestion) => {
		setPrefill(s);
		setShowForm(true);
		window.scrollTo({ top: 0, behavior: "smooth" });
	};

	return {
		busy,
		run,
		save,
		handleRefresh,
		handleDelete,
		handleLink,
		handleAddFromLog,
	};
}
