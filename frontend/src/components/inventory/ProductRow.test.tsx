// @vitest-environment jsdom
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { Product } from "../../types/inventory";
import ProductRow, { type Suggestion } from "./ProductRow";
import { makeProduct, makeProfile } from "./testFixtures";

vi.mock("../../api/client", () => ({ updateProduct: vi.fn() }));

function row(product: Product, expanded: boolean, suggestion?: Suggestion) {
	const handlers = {
		onToggle: vi.fn(),
		onStatus: vi.fn(),
		onEdit: vi.fn(),
		onSaved: vi.fn(),
		onRefresh: vi.fn(),
		onArchive: vi.fn(),
		onDelete: vi.fn(),
		onLink: vi.fn(),
		onDismissSuggestion: vi.fn(),
		onError: vi.fn(),
	};
	render(
		<ProductRow
			product={product}
			expanded={expanded}
			editing={false}
			suggestion={suggestion}
			busy={null}
			{...handlers}
		/>,
	);
	return handlers;
}

const fungicide = makeProduct({
	name: "Heritage G",
	brand: "Syngenta",
	category: "Fungicide",
	frac_classes: ["Frac11", "FracM5"],
	profile: makeProfile(),
	profile_generated_at: "2026-09-01T12:00:00Z",
	profile_model: "anthropic/claude-haiku-4.5",
});

// @spec INV-UI-005
describe("ProductRow", () => {
	it("shows the header facts and the summary while collapsed", () => {
		row(
			makeProduct({
				name: "Mix",
				brand: "Acme",
				category: "Other",
				nitrogen_pct: 24,
				potassium_pct: 6,
				frac_classes: ["Frac3"],
				herbicide_timing: "PreEmergent",
				amendment_kind: "Gypsum",
				targets: ["Grubs", "Iron"],
				stock_status: "Low",
				profile: makeProfile({ summary: "A catch-all." }),
			}),
			false,
		);
		for (const text of [
			"Mix",
			"Acme",
			"Granular",
			"24-0-6",
			"FRAC 3",
			"Pre-emergent",
			"Gypsum",
			"A catch-all.",
		])
			expect(screen.getByText(text)).toBeVisible();
		expect(screen.getByRole("button", { name: /running low/i })).toBeVisible();
		expect(screen.queryByRole("button", { name: "Edit" })).toBeNull();
	});

	it("shows provenance and every action when expanded", async () => {
		const h = row(fungicide, true);
		expect(screen.getByText("FRAC 11, M5")).toBeVisible();
		expect(
			screen.getByText(/Profile generated .* · anthropic\/claude-haiku-4.5/),
		).toBeVisible();
		for (const [name, handler] of [
			["Edit", h.onEdit],
			["Regenerate profile", h.onRefresh],
			["Link past applications", h.onLink],
			["Archive", h.onArchive],
			["Delete", h.onDelete],
		] as const) {
			await userEvent.click(screen.getByRole("button", { name }));
			expect(handler).toHaveBeenCalled();
		}
	});

	it("says a hand-entered product has no profile and offers to generate one", () => {
		row(makeProduct({ archived: true }), true);
		expect(
			screen.getByText("Entered by hand — no assistant profile"),
		).toBeVisible();
		expect(
			screen.getByRole("button", { name: "Generate profile" }),
		).toBeVisible();
		expect(screen.getByRole("button", { name: "Unarchive" })).toBeVisible();
	});
});

// @spec INV-UI-008
describe("ProductRow refresh suggestion", () => {
	it("names the suggested changes and offers Review in editor or Ignore", async () => {
		const h = row(fungicide, true, {
			facts: { ...fungicide, form: "Liquid" },
			changed: ["form", "label_rate", "frac_classes"],
		});
		expect(
			screen.getByText(
				/The refreshed profile suggests changes to form, suggested rate, FRAC classes/,
			),
		).toBeVisible();
		await userEvent.click(
			screen.getByRole("button", { name: "Review in editor" }),
		);
		expect(h.onEdit).toHaveBeenCalledWith(true);
		await userEvent.click(screen.getByRole("button", { name: "Ignore" }));
		expect(h.onDismissSuggestion).toHaveBeenCalled();
	});
});
