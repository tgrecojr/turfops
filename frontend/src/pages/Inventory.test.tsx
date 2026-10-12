// @vitest-environment jsdom
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router";
import { beforeEach, describe, expect, it, vi } from "vitest";
import * as client from "../api/client";
import { makeProduct } from "../components/inventory/testFixtures";
import type { Product } from "../types/inventory";
import Inventory from "./Inventory";

vi.mock("../api/client", () => ({
	listProducts: vi.fn(),
	createProduct: vi.fn(),
	updateProduct: vi.fn(),
	deleteProduct: vi.fn(),
	refreshProductProfile: vi.fn(),
	linkProductApplications: vi.fn(),
	listLoggedProductSuggestions: vi.fn(),
	getShoppingList: vi.fn(),
}));
const api = vi.mocked(client);

function page(products: Product[]) {
	api.listProducts.mockResolvedValue(products);
	render(
		<MemoryRouter>
			<Inventory />
		</MemoryRouter>,
	);
}

beforeEach(() => {
	vi.resetAllMocks();
	api.listLoggedProductSuggestions.mockResolvedValue([]);
	api.getShoppingList.mockResolvedValue({ generated_at: "", items: [] });
});

// @spec INV-UI-001
describe("Inventory shelf", () => {
	it("groups by category in order, sorts by name, and hides archived behind a toggle", async () => {
		page([
			makeProduct({ id: 1, name: "zeta seed", category: "Seed" }),
			makeProduct({ id: 2, name: "Banner", category: "Fungicide" }),
			makeProduct({ id: 3, name: "beta", category: "Fertilizer" }),
			makeProduct({ id: 4, name: "Alpha", category: "Fertilizer" }),
			makeProduct({
				id: 5,
				name: "Old bag",
				category: "Fertilizer",
				archived: true,
			}),
		]);
		const headings = await screen.findAllByRole("heading", { level: 2 });
		expect(headings.map((h) => h.textContent)).toEqual([
			"Fertilizer",
			"Fungicide",
			"Seed",
		]);
		const fert = headings[0].parentElement as HTMLElement;
		const names = within(fert)
			.getAllByRole("button", { expanded: false })
			.map((b) => b.textContent);
		expect(names[0]).toMatch(/^.Alpha/);
		expect(names[1]).toMatch(/^.beta/);
		expect(screen.queryByText("Old bag")).toBeNull();
		await userEvent.click(
			screen.getByRole("button", { name: "Show 1 archived" }),
		);
		expect(screen.getByText("Old bag")).toBeVisible();
		expect(api.listProducts).toHaveBeenCalledWith({ includeArchived: true });
	});

	it("says how to start when the shelf is empty", async () => {
		page([]);
		expect(
			await screen.findByText(
				'Nothing on the shelf yet. Click "+ Add Product" to get started.',
			),
		).toBeVisible();
	});
});

// @spec INV-UI-004
describe("Inventory after a hand save", () => {
	it("says the product was saved by hand and Generate profile can be tried later", async () => {
		page([]);
		api.createProduct.mockResolvedValue({
			product: makeProduct({ name: "Heritage G" }),
			profile_error: "The assistant couldn't identify this product.",
		});
		await userEvent.click(
			await screen.findByRole("button", { name: "+ Add Product" }),
		);
		await userEvent.type(screen.getByLabelText(/Product name/), "Heritage G");
		await userEvent.click(
			screen.getByRole("button", { name: "Add & identify" }),
		);
		expect(await screen.findByText(/Saved by hand/)).toHaveTextContent(
			/try "Generate profile" later/,
		);
	});
});

// @spec INV-UI-006
describe("Inventory stock pill", () => {
	it("saves the next stock status when the pill is clicked", async () => {
		const p = makeProduct({ id: 7, stock_status: "Low" });
		page([p]);
		api.updateProduct.mockResolvedValue({ ...p, stock_status: "Out" });
		await userEvent.click(
			await screen.findByRole("button", { name: /Running low/ }),
		);
		expect(api.updateProduct).toHaveBeenCalledWith(
			7,
			expect.objectContaining({ stock_status: "Out", name: p.name }),
		);
		const pill = await screen.findByRole("button", { name: "Out" });
		expect(pill).toHaveTextContent("✕ Out");
	});
});
