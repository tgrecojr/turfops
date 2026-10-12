// @vitest-environment jsdom
import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { getShoppingList } from "../../api/client";
import { ApiError } from "../../api/errors";
import type { ShoppingItem } from "../../types/inventory";
import ShoppingListPanel from "./ShoppingListPanel";

vi.mock("../../api/client", () => ({ getShoppingList: vi.fn() }));
const list = vi.mocked(getShoppingList);

beforeEach(() => {
	list.mockClear();
});

const item = (label: string): ShoppingItem => ({
	kind: "Buy",
	label,
	categories: ["Fertilizer"],
	product: null,
	reasons: ["Fall Feeding"],
	urgency: "Warning",
});

function show(compact: boolean) {
	return render(
		<MemoryRouter>
			<ShoppingListPanel compact={compact} />
		</MemoryRouter>,
	);
}

// @spec INV-UI-012
describe("dashboard shopping list", () => {
	it("shows the first 4 items with n more and View all to the inventory", async () => {
		const items = ["a", "b", "c", "d", "e", "f"].map(item);
		list.mockResolvedValue({ generated_at: "", items });
		show(true);
		expect(await screen.findByText("a")).toBeVisible();
		expect(screen.getByText("d")).toBeVisible();
		expect(screen.queryByText("e")).toBeNull();
		expect(screen.getByRole("link", { name: /2 more/ })).toHaveAttribute(
			"href",
			"/inventory",
		);
		expect(screen.getByRole("link", { name: /View all/ })).toHaveAttribute(
			"href",
			"/inventory",
		);
	});

	it("is hidden while loading, when empty and when the fetch fails", async () => {
		list.mockReturnValue(new Promise(() => {}));
		const loading = show(true);
		expect(loading.container).toBeEmptyDOMElement();
		loading.unmount();

		list.mockResolvedValue({ generated_at: "", items: [] });
		const empty = show(true);
		await vi.waitFor(() => expect(list).toHaveBeenCalledTimes(2));
		expect(empty.container).toBeEmptyDOMElement();
		empty.unmount();

		list.mockRejectedValue(
			new ApiError(503, "The weather data lake can't be read."),
		);
		const failed = show(true);
		await vi.waitFor(() => expect(list).toHaveBeenCalledTimes(3));
		expect(failed.container).toBeEmptyDOMElement();
	});
});

// @spec INV-UI-013
describe("shopping list failure", () => {
	it("shows the server's plain message, never the status or body", async () => {
		list.mockRejectedValue(
			new ApiError(503, "The weather data lake can't be read right now."),
		);
		show(false);
		expect(
			await screen.findByText("The weather data lake can't be read right now."),
		).toBeVisible();
		expect(screen.queryByText(/503/)).toBeNull();
	});

	it("asks the owner to try again after a timeout", async () => {
		list.mockImplementation(() =>
			Promise.reject(new ApiError(0, "The request timed out.", true)),
		);
		show(false);
		expect(await screen.findByText(/Try again/)).toBeVisible();
	});
});
