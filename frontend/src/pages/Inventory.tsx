import { useCallback, useEffect, useMemo, useState } from "react";
import { listProducts } from "../api/client";
import AddProductForm from "../components/inventory/AddProductForm";
import LogSuggestions from "../components/inventory/LogSuggestions";
import ProductRow, {
	type Suggestion,
} from "../components/inventory/ProductRow";
import ShoppingListPanel from "../components/inventory/ShoppingListPanel";
import { inv } from "../components/inventory/styles";
import { useProductActions } from "../components/inventory/useProductActions";
import { sharedStyles } from "../styles/shared";
import type {
	LoggedProductSuggestion,
	Product,
	ProductCategory,
	ProductCreated,
} from "../types/inventory";
import {
	nextStockStatus,
	PRODUCT_CATEGORIES,
	PRODUCT_CATEGORY_LABELS,
} from "../types/inventory";
import { guessCategory } from "../types/inventoryCompat";

export default function Inventory() {
	const [products, setProducts] = useState<Product[]>([]);
	const [loading, setLoading] = useState(true);
	const [error, setError] = useState<string | null>(null);
	const [notice, setNotice] = useState<string | null>(null);
	const [showForm, setShowForm] = useState(false);
	const [showArchived, setShowArchived] = useState(false);
	const [expanded, setExpanded] = useState<Set<number>>(new Set());
	const [editingId, setEditingId] = useState<number | null>(null);
	const [suggestions, setSuggestions] = useState<Map<number, Suggestion>>(
		new Map(),
	);
	const [prefill, setPrefill] = useState<LoggedProductSuggestion | null>(null);
	const [logVersion, setLogVersion] = useState(0);

	const fetchProducts = useCallback(async () => {
		try {
			setProducts(await listProducts({ includeArchived: true }));
			setError(null);
		} catch (e) {
			setError(e instanceof Error ? e.message : "Failed to load inventory");
		} finally {
			setLoading(false);
		}
	}, []);

	useEffect(() => {
		fetchProducts();
	}, [fetchProducts]);

	const grouped = useMemo(() => {
		const visible = products.filter((p) => showArchived || !p.archived);
		return PRODUCT_CATEGORIES.map((c) => ({
			category: c,
			items: visible
				.filter((p) => p.category === c)
				.sort((a, b) =>
					a.name.localeCompare(b.name, undefined, { sensitivity: "base" }),
				),
		})).filter((g) => g.items.length > 0);
	}, [products, showArchived]);

	const archivedCount = products.filter((p) => p.archived).length;
	// Anything that changes what the shelf covers re-derives the shopping list.
	const shelfVersion = useMemo(
		() =>
			products
				.map((p) => `${p.id}:${p.stock_status}:${p.archived}:${p.updated_at}`)
				.join("|"),
		[products],
	);

	const replace = (updated: Product) =>
		setProducts((prev) => prev.map((p) => (p.id === updated.id ? updated : p)));

	const toggleExpanded = (id: number) =>
		setExpanded((prev) => {
			const next = new Set(prev);
			if (next.has(id)) next.delete(id);
			else next.add(id);
			return next;
		});

	const handleCreated = (created: ProductCreated) => {
		setShowForm(false);
		setPrefill(null);
		setLogVersion((v) => v + 1);
		setProducts((prev) => [...prev, created.product]);
		if (created.product.id != null) {
			const id = created.product.id;
			setExpanded((prev) => new Set(prev).add(id));
		}
		setNotice(
			created.profile_error
				? `Saved by hand — the assistant could not identify it (${created.profile_error}). You can try "Generate profile" later.`
				: null,
		);
	};

	const actions = useProductActions({
		setProducts,
		setError,
		setNotice,
		setSuggestions,
		setExpanded,
		setLogVersion,
		setPrefill,
		setShowForm,
	});
	const {
		busy,
		run,
		save,
		handleRefresh,
		handleDelete,
		handleLink,
		handleAddFromLog,
	} = actions;

	const dismissSuggestion = (id: number) =>
		setSuggestions((prev) => {
			const next = new Map(prev);
			next.delete(id);
			return next;
		});

	return (
		<div>
			<div style={sharedStyles.headerRow}>
				<h1 style={sharedStyles.pageTitle}>Inventory</h1>
				<button
					type="button"
					style={styles.addBtn}
					onClick={() => setShowForm(!showForm)}
				>
					{showForm ? "Cancel" : "+ Add Product"}
				</button>
			</div>

			<p style={styles.intro}>
				What's on the shelf: fertilizer, supplements, fungicides, herbicides,
				insect control, seed, amendments and surfactants. Click a stock pill to
				mark a product low or out. Recommendations will say whether you have
				what they call for.
			</p>

			{showForm && (
				<AddProductForm
					key={prefill?.name ?? "blank"}
					initialName={prefill?.name}
					initialCategory={
						prefill ? guessCategory(prefill.application_types) : undefined
					}
					onCreated={handleCreated}
					onError={setError}
				/>
			)}
			{notice && <div style={inv.warnBox}>{notice}</div>}
			{error && <div style={sharedStyles.error}>{error}</div>}
			{!loading && <ShoppingListPanel key={shelfVersion} />}
			{!loading && <LogSuggestions key={logVersion} onAdd={handleAddFromLog} />}

			{loading ? (
				<p style={sharedStyles.loading}>Loading inventory...</p>
			) : products.length === 0 ? (
				<p style={sharedStyles.empty}>
					Nothing on the shelf yet. Click "+ Add Product" to get started.
				</p>
			) : (
				<div style={styles.list}>
					{grouped.map((g) => (
						<section key={g.category}>
							<h2 style={styles.groupTitle}>
								{PRODUCT_CATEGORY_LABELS[g.category as ProductCategory]}
							</h2>
							<div style={styles.group}>
								{g.items.map((p) => (
									<ProductRow
										key={p.id ?? p.name}
										product={p}
										expanded={p.id != null && expanded.has(p.id)}
										editing={editingId === p.id}
										suggestion={
											p.id != null ? suggestions.get(p.id) : undefined
										}
										busy={busy?.id === p.id ? busy.what : null}
										onToggle={() => p.id != null && toggleExpanded(p.id)}
										onStatus={() =>
											run(p.id ?? -1, "status", () =>
												save(p, {
													stock_status: nextStockStatus(p.stock_status),
												}),
											)
										}
										onEdit={(open) => setEditingId(open ? p.id : null)}
										onSaved={(updated) => {
											replace(updated);
											setEditingId(null);
											if (updated.id != null) dismissSuggestion(updated.id);
										}}
										onRefresh={() => handleRefresh(p)}
										onArchive={() =>
											run(p.id ?? -1, "archive", () =>
												save(p, { archived: !p.archived }),
											)
										}
										onDelete={() => handleDelete(p)}
										onLink={() => handleLink(p)}
										onDismissSuggestion={() =>
											p.id != null && dismissSuggestion(p.id)
										}
										onError={setError}
									/>
								))}
							</div>
						</section>
					))}
					{archivedCount > 0 && (
						<button
							type="button"
							style={styles.linkBtn}
							onClick={() => setShowArchived(!showArchived)}
						>
							{showArchived ? "Hide" : "Show"} {archivedCount} archived
						</button>
					)}
				</div>
			)}
		</div>
	);
}

const styles: Record<string, React.CSSProperties> = {
	intro: { color: "#4a5568", fontSize: "0.9rem", marginBottom: "1rem" },
	addBtn: {
		padding: "0.5rem 1rem",
		backgroundColor: "#48bb78",
		color: "#fff",
		border: "none",
		borderRadius: 6,
		cursor: "pointer",
		fontWeight: 600,
		fontSize: "0.85rem",
	},
	list: {
		display: "flex",
		flexDirection: "column",
		gap: "1rem",
		maxWidth: 960,
	},
	groupTitle: {
		fontSize: "0.8rem",
		color: "#718096",
		fontWeight: 600,
		textTransform: "uppercase",
		margin: "0 0 0.5rem",
	},
	group: { display: "flex", flexDirection: "column", gap: "0.5rem" },
	linkBtn: {
		alignSelf: "flex-start",
		background: "none",
		border: "none",
		color: "#3182ce",
		cursor: "pointer",
		fontSize: "0.8rem",
		padding: 0,
	},
};
