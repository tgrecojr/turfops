// @vitest-environment jsdom
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router";
import { beforeEach, describe, expect, it, vi } from "vitest";
import * as client from "../api/client";
import { ApiError } from "../api/errors";
import { makeProduct, makeProfile } from "../components/inventory/testFixtures";
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

const heritage = makeProduct({
	id: 3,
	name: "Heritage G",
	category: "Fungicide",
	profile: makeProfile(),
	profile_generated_at: "2026-09-01T12:00:00Z",
	profile_model: "test/model",
});

async function expandedPage(products: Product[]) {
	api.listProducts.mockResolvedValue(products);
	render(
		<MemoryRouter>
			<Inventory />
		</MemoryRouter>,
	);
	await userEvent.click(await screen.findByRole("button", { expanded: false }));
}

beforeEach(() => {
	vi.resetAllMocks();
	api.listLoggedProductSuggestions.mockResolvedValue([]);
	api.getShoppingList.mockResolvedValue({ generated_at: "", items: [] });
});

// @spec INV-LOG-004
describe("Link past applications", () => {
	it.each([
		[2, "Linked 2 past applications to Heritage G."],
		[1, "Linked 1 past application to Heritage G."],
		[0, 'No unlinked applications named "Heritage G" were found.'],
	])(
		"reports %i linked and refreshes the log suggestions",
		async (linked, text) => {
			await expandedPage([heritage]);
			api.linkProductApplications.mockResolvedValue({ linked });
			const before = api.listLoggedProductSuggestions.mock.calls.length;
			await userEvent.click(
				screen.getByRole("button", { name: "Link past applications" }),
			);
			expect(await screen.findByText(text)).toBeVisible();
			expect(api.linkProductApplications).toHaveBeenCalledWith(3);
			await vi.waitFor(() =>
				expect(
					api.listLoggedProductSuggestions.mock.calls.length,
				).toBeGreaterThan(before),
			);
		},
	);
});

// @spec INV-UI-008
describe("Profile refresh suggestions", () => {
	it("opens the editor on the suggested facts and saves nothing until asked", async () => {
		await expandedPage([heritage]);
		api.refreshProductProfile.mockResolvedValue({
			product: heritage,
			suggested_facts: { ...heritage, category: "Herbicide", form: "Liquid" },
			changed_fields: ["category", "form"],
		});
		await userEvent.click(
			screen.getByRole("button", { name: "Regenerate profile" }),
		);
		expect(
			await screen.findByText(
				/The refreshed profile suggests changes to category, form/,
			),
		).toBeVisible();
		await userEvent.click(
			screen.getByRole("button", { name: "Review in editor" }),
		);
		expect(screen.getByDisplayValue("Herbicide")).toBeVisible();
		expect(screen.getByDisplayValue("Liquid")).toBeVisible();
		expect(api.updateProduct).not.toHaveBeenCalled();
	});
});

// @spec INV-LOG-003
describe("Add to shelf from the log", () => {
	it("opens the add form with the name and its first specific category", async () => {
		api.listLoggedProductSuggestions.mockResolvedValue([
			{
				name: "GrubEx",
				uses: 2,
				last_used: "2026-06-01",
				application_types: ["Mowing", "GrubControl"],
			},
		]);
		window.scrollTo = vi.fn();
		api.listProducts.mockResolvedValue([]);
		render(
			<MemoryRouter>
				<Inventory />
			</MemoryRouter>,
		);
		await userEvent.click(
			await screen.findByRole("button", { name: "Add to shelf" }),
		);
		expect(screen.getByLabelText(/Product name/)).toHaveValue("GrubEx");
		expect(screen.getByLabelText(/Category/)).toHaveValue("InsectControl");
	});
});

// @spec INV-UI-013
describe("Inventory request failures", () => {
	it("shows the server's sentence for a failed list, not the status", async () => {
		api.listProducts.mockRejectedValue(
			new ApiError(500, "Something went wrong on the server. Try again."),
		);
		render(
			<MemoryRouter>
				<Inventory />
			</MemoryRouter>,
		);
		expect(
			await screen.findByText(/Something went wrong on the server/),
		).toBeVisible();
		expect(screen.queryByText(/500/)).toBeNull();
	});

	it("names the unavailable assistant when a refresh is a 503", async () => {
		await expandedPage([heritage]);
		api.refreshProductProfile.mockRejectedValue(
			new ApiError(503, "The product assistant isn't configured."),
		);
		await userEvent.click(
			screen.getByRole("button", { name: "Regenerate profile" }),
		);
		expect(
			await screen.findByText("The product assistant isn't configured."),
		).toBeVisible();
	});

	it("points to Settings when there is no lawn profile", async () => {
		api.listProducts.mockRejectedValue(
			new ApiError(404, "No lawn profile found"),
		);
		render(
			<MemoryRouter>
				<Inventory />
			</MemoryRouter>,
		);
		expect(await screen.findByText(/Settings/)).toBeVisible();
	});
});
