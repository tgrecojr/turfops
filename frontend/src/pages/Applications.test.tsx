// @vitest-environment jsdom
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import * as client from "../api/client";
import { ApiError } from "../api/errors";
import type { Application } from "../types";
import Applications from "./Applications";

vi.mock("../api/client", () => ({
	getApplications: vi.fn(),
	listPlants: vi.fn(),
	listProducts: vi.fn(),
	getFracClasses: vi.fn(),
	getToday: vi.fn(),
	createApplication: vi.fn(),
	updateApplication: vi.fn(),
	deleteApplication: vi.fn(),
	markProductInStock: vi.fn(),
}));

const api = vi.mocked(client);

function entry(id: number, extra: Partial<Application> = {}): Application {
	return {
		id,
		lawn_profile_id: 1,
		application_type: "Fertilizer",
		product_name: "Milorganite",
		application_date: "2026-05-01",
		rate_per_1000sqft: 32,
		coverage_sqft: 5000,
		notes: "front",
		nitrogen_pct: 6,
		phosphorus_pct: 4,
		potassium_pct: 0,
		created_at: "2026-05-01T12:00:00Z",
		...extra,
	};
}

beforeEach(() => {
	api.listPlants.mockResolvedValue([]);
	api.listProducts.mockResolvedValue([]);
	api.getFracClasses.mockResolvedValue({ options: [], matched: [] });
	api.getToday.mockResolvedValue({ today: "2026-10-11", timezone: null });
});

afterEach(() => {
	vi.clearAllMocks();
	vi.restoreAllMocks();
});

describe("Applications list", () => {
	// @spec APP-LIST-001
	it("lists each entry with its target, numbers and row actions", async () => {
		api.listPlants.mockResolvedValue([
			{ id: 7, common_name: "Viburnum" } as never,
		]);
		api.getApplications.mockResolvedValue([
			entry(1, { follow_up_date: "2026-05-29" }),
			entry(2, {
				application_type: "Pruning",
				plant_id: 7,
				product_name: null,
				rate_per_1000sqft: null,
				nitrogen_pct: null,
				coverage_sqft: null,
				notes: null,
			}),
		]);
		render(<Applications />);

		const table = await screen.findByRole("table");
		const headers = within(table)
			.getAllByRole("columnheader")
			.map((h) => h.textContent);
		expect(headers).toEqual([
			"Date",
			"Type",
			"Target",
			"Product",
			"Rate/1k sqft",
			"N-P-K",
			"Coverage",
			"Follow-up",
			"Notes",
			"",
		]);
		const [, lawnRow, shrubRow] = within(table).getAllByRole("row");
		for (const text of [
			"2026-05-01",
			"Fertilizer",
			"Turf",
			"Milorganite",
			"32.00",
			"6-4-0",
			"5,000 sqft",
			"2026-05-29",
			"front",
		]) {
			expect(lawnRow).toHaveTextContent(text);
		}
		await waitFor(() => expect(shrubRow).toHaveTextContent("Viburnum"));
		expect(shrubRow).toHaveTextContent("Pruning");
		expect(
			within(lawnRow).getByRole("button", { name: "Edit" }),
		).toBeInTheDocument();
		expect(
			within(lawnRow).getByRole("button", { name: "Delete" }),
		).toBeInTheDocument();
	});

	// @spec APP-LIST-002
	it("loads 50 at a time and offers older entries while a page is full", async () => {
		const page = (start: number, n: number) =>
			Array.from({ length: n }, (_, i) => entry(start + i));
		api.getApplications
			.mockResolvedValueOnce(page(1, 50))
			.mockResolvedValueOnce(page(51, 3));
		render(<Applications />);

		const older = await screen.findByRole("button", {
			name: "Load older entries",
		});
		expect(api.getApplications).toHaveBeenLastCalledWith(
			undefined,
			50,
			0,
			undefined,
		);
		await userEvent.click(older);
		await waitFor(() => expect(screen.getAllByRole("row")).toHaveLength(54));
		expect(api.getApplications).toHaveBeenLastCalledWith(
			undefined,
			50,
			50,
			undefined,
		);
		expect(
			screen.queryByRole("button", { name: "Load older entries" }),
		).not.toBeInTheDocument();
	});

	// @spec APP-LIST-004
	it("asks before deleting and deletes only on yes", async () => {
		api.getApplications.mockResolvedValue([entry(1)]);
		api.deleteApplication.mockResolvedValue(undefined);
		const confirm = vi.spyOn(window, "confirm").mockReturnValueOnce(false);
		render(<Applications />);

		const remove = await screen.findByRole("button", { name: "Delete" });
		await userEvent.click(remove);
		expect(confirm).toHaveBeenCalledTimes(1);
		expect(api.deleteApplication).not.toHaveBeenCalled();

		confirm.mockReturnValueOnce(true);
		await userEvent.click(remove);
		expect(api.deleteApplication).toHaveBeenCalledWith(1);
		await waitFor(() =>
			expect(screen.getByText("No applications found.")).toBeInTheDocument(),
		);
	});

	// @spec APP-LIST-005
	it("says what went wrong and what to do, never the raw response", async () => {
		api.getApplications.mockRejectedValueOnce(
			new ApiError(404, "No lawn profile found"),
		);
		const { unmount } = render(<Applications />);
		const missing = await screen.findByText(/Set one up in Settings/);
		expect(missing).not.toHaveTextContent("404");
		unmount();

		api.getApplications.mockRejectedValueOnce(new ApiError(0, "timeout", true));
		render(<Applications />);
		const slow = await screen.findByText(/Try again/);
		expect(slow).not.toHaveTextContent("timeout");
	});

	// @spec APP-LIST-005
	it("shows the server's own message for a rejected save", async () => {
		api.getApplications.mockResolvedValue([]);
		const duplicate =
			"An identical entry (same type, product and date) is already logged.";
		api.createApplication.mockRejectedValue(new ApiError(409, duplicate));
		render(<Applications />);

		await userEvent.click(
			await screen.findByRole("button", { name: "+ Add Application" }),
		);
		await userEvent.click(
			screen.getByRole("button", { name: "Save Application" }),
		);
		const shown = await screen.findByText(duplicate);
		expect(shown).not.toHaveTextContent("409");
		expect(shown.textContent).not.toMatch(/[{}]/);
	});
});
