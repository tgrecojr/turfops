// @vitest-environment jsdom
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import * as client from "../api/client";
import type { ApplicationType } from "../types";
import type { Product } from "../types/inventory";
import ProductPicker from "./ProductPicker";

vi.mock("../api/client", () => ({ listProducts: vi.fn() }));

const api = vi.mocked(client);

function product(id: number, extra: Partial<Product>): Product {
	return {
		id,
		lawn_profile_id: 1,
		name: `Product ${id}`,
		brand: null,
		stock_status: "InStock",
		category: "Fertilizer",
		form: "Granular",
		label_rate_per_1000sqft: null,
		label_rate_unit: null,
		nitrogen_pct: null,
		phosphorus_pct: null,
		potassium_pct: null,
		frac_classes: [],
		herbicide_timing: null,
		targets: [],
		amendment_kind: null,
		profile: null,
		profile_generated_at: null,
		profile_model: null,
		notes: null,
		archived: false,
		created_at: "2026-01-01T00:00:00Z",
		updated_at: "2026-01-01T00:00:00Z",
		...extra,
	};
}

const SHELF = [
	product(1, { name: "Milorganite", brand: "MMSD" }),
	product(2, { name: "Lesco 24-0-11", stock_status: "Low" }),
	product(3, { name: "Old Starter", archived: true }),
	product(4, { name: "Heritage G", category: "Fungicide" }),
	product(5, { name: "Mystery Mix", category: "Other" }),
	product(6, { name: "Scotts Turf Builder", stock_status: "Out" }),
];

function picker(
	appType: ApplicationType,
	productId: number | null = null,
	productName = "",
) {
	const onPick = vi.fn();
	const onNameChange = vi.fn();
	render(
		<ProductPicker
			appType={appType}
			productId={productId}
			productName={productName}
			onPick={onPick}
			onNameChange={onNameChange}
			inputStyle={{}}
			labelStyle={{}}
		/>,
	);
	return { onPick, onNameChange };
}

async function offered(): Promise<string[]> {
	const select = await screen.findByLabelText("Product");
	await waitFor(() => expect(select).toBeEnabled());
	return within(select)
		.getAllByRole("option")
		.map((o) => o.textContent ?? "");
}

afterEach(() => vi.clearAllMocks());

describe("product picker", () => {
	// @spec APP-FORM-006
	it("offers the usable products this type accepts, labeled with brand and state", async () => {
		api.listProducts.mockResolvedValue(SHELF);
		picker("Fertilizer");
		expect(await offered()).toEqual([
			"— none —",
			"Milorganite (MMSD)",
			"Lesco 24-0-11 · Running low",
			"Mystery Mix · Other",
			"Scotts Turf Builder · Out",
			"Not in inventory — type a name…",
		]);
		expect(api.listProducts).toHaveBeenCalledWith({ includeArchived: true });
	});

	// @spec APP-FORM-006
	it("keeps the linked product even when archived or no longer compatible", async () => {
		api.listProducts.mockResolvedValue(SHELF);
		picker("Fungicide", 3, "Old Starter");
		const options = await offered();
		expect(options).toContain("Old Starter · archived");
		expect(options).toContain("Heritage G");
		expect(options).not.toContain("Milorganite (MMSD)");
	});

	// @spec APP-FORM-007
	it("clears the link and name with none, and reveals a name field for a product not on the shelf", async () => {
		api.listProducts.mockResolvedValue(SHELF);
		const { onPick, onNameChange } = picker("Fertilizer", 1, "Milorganite");
		await offered();
		expect(screen.queryByLabelText("Product Name")).not.toBeInTheDocument();

		await userEvent.selectOptions(screen.getByLabelText("Product"), "");
		expect(onPick).toHaveBeenLastCalledWith(null);
		expect(onNameChange).toHaveBeenLastCalledWith("");

		await userEvent.selectOptions(
			screen.getByLabelText("Product"),
			"Not in inventory — type a name…",
		);
		expect(onPick).toHaveBeenLastCalledWith(null);
		expect(screen.getByLabelText("Product Name")).toBeInTheDocument();

		await userEvent.selectOptions(screen.getByLabelText("Product"), "2");
		expect(onPick).toHaveBeenLastCalledWith(SHELF[1]);
	});

	// @spec APP-FORM-007
	it.each([
		["empty", () => api.listProducts.mockResolvedValue([])],
		["unreadable", () => api.listProducts.mockRejectedValue(new Error("down"))],
	])("shows only the free-text name when the shelf is %s", async (_, shelf) => {
		shelf();
		picker("Fertilizer");
		expect(await screen.findByLabelText("Product Name")).toBeInTheDocument();
		expect(screen.queryByLabelText("Product")).not.toBeInTheDocument();
	});
});
