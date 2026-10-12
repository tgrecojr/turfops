// @vitest-environment jsdom
import {
	act,
	fireEvent,
	render,
	screen,
	waitFor,
} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import * as client from "../api/client";
import type { Product } from "../types/inventory";
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

function product(id: number, extra: Partial<Product> = {}): Product {
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

const FRAC_OPTIONS = [
	{ id: "Frac3", label: "FRAC 3 — DMIs", examples: ["Banner Maxx"] },
	{ id: "Frac11", label: "FRAC 11 — Strobilurins", examples: ["Heritage G"] },
];

beforeEach(() => {
	api.getApplications.mockResolvedValue([]);
	api.listPlants.mockResolvedValue([]);
	api.listProducts.mockResolvedValue([]);
	api.getFracClasses.mockResolvedValue({ options: FRAC_OPTIONS, matched: [] });
	api.getToday.mockResolvedValue({ today: "2026-10-11", timezone: null });
	api.createApplication.mockResolvedValue({} as never);
	api.markProductInStock.mockResolvedValue({} as never);
});

afterEach(() => {
	vi.useRealTimers();
	vi.clearAllMocks();
	vi.restoreAllMocks();
});

async function openForm() {
	render(<Applications />);
	await userEvent.click(
		await screen.findByRole("button", { name: "+ Add Application" }),
	);
}

/** Submit the form; returns the payload this save sent. */
async function save() {
	const before = api.createApplication.mock.calls.length;
	await userEvent.click(
		screen.getByRole("button", { name: "Save Application" }),
	);
	await waitFor(() =>
		expect(api.createApplication).toHaveBeenCalledTimes(before + 1),
	);
	return api.createApplication.mock.calls[before][0];
}

describe("application form", () => {
	// @spec APP-FORM-001
	it("defaults a new entry to today on the local calendar", async () => {
		// Late evening west of UTC (or just after midnight east of it), the UTC date is
		// already another day.
		const west = new Date(2026, 9, 11).getTimezoneOffset() > 0;
		vi.useFakeTimers({ toFake: ["Date"] });
		vi.setSystemTime(new Date(2026, 9, 11, west ? 23 : 0, 30));
		await openForm();
		expect(screen.getByLabelText("Date")).toHaveValue("2026-10-11");
	});

	// @spec APP-FORM-003
	it("schedules a follow-up no earlier than the date, with week shortcuts", async () => {
		await openForm();
		fireEvent.change(screen.getByLabelText("Date"), {
			target: { value: "2026-10-25" },
		});
		await userEvent.click(screen.getByLabelText("Schedule a follow-up"));
		const followUp = document.querySelector<HTMLInputElement>(
			'input[type="date"][min]',
		);
		expect(followUp).toHaveAttribute("min", "2026-10-25");
		// Across the end of daylight saving time, still whole calendar days.
		for (const [label, expected] of [
			["+2 wk", "2026-11-08"],
			["+4 wk", "2026-11-22"],
			["+6 wk", "2026-12-06"],
		]) {
			await userEvent.click(screen.getByRole("button", { name: label }));
			expect(followUp).toHaveValue(expected);
		}
	});

	// @spec APP-FORM-005
	it("sends empty fields as absent", async () => {
		await openForm();
		const payload = await save();
		expect(payload).toMatchObject({ application_type: "Fertilizer" });
		for (const key of [
			"product_name",
			"rate_per_1000sqft",
			"coverage_sqft",
			"notes",
			"nitrogen_pct",
			"phosphorus_pct",
			"potassium_pct",
			"plant_id",
			"follow_up_date",
			"frac_classes",
		] as const) {
			expect(payload?.[key], key).toBeUndefined();
		}
	});

	// @spec APP-FORM-005, APP-FRAC-002
	it("sends FRAC classes only for a fungicide with a class ticked", async () => {
		const reopen = async () =>
			userEvent.click(
				await screen.findByRole("button", { name: "+ Add Application" }),
			);
		const asFungicide = async () => {
			await userEvent.selectOptions(screen.getByLabelText("Type"), "Fungicide");
			return screen.findByLabelText("FRAC 11 — Strobilurins");
		};
		await openForm();
		expect(screen.queryByText(/FRAC class/)).not.toBeInTheDocument();
		const strobilurin = await asFungicide();
		expect(strobilurin.closest("label")).toHaveAttribute("title", "Heritage G");
		expect((await save())?.frac_classes).toBeUndefined();

		await reopen();
		await userEvent.click(await asFungicide());
		expect((await save())?.frac_classes).toEqual(["Frac11"]);

		await reopen();
		await userEvent.click(await asFungicide());
		await userEvent.selectOptions(screen.getByLabelText("Type"), "Fertilizer");
		expect(screen.queryByText(/FRAC class/)).not.toBeInTheDocument();
		expect((await save())?.frac_classes).toBeUndefined();
	});

	// @spec APP-FORM-008
	it("pre-fills from the picked shelf product and keeps every field editable", async () => {
		api.listProducts.mockResolvedValue([
			product(5, {
				name: "Milorganite",
				nitrogen_pct: 6,
				phosphorus_pct: 4,
				potassium_pct: 0,
				label_rate_per_1000sqft: 32,
			}),
		]);
		await openForm();
		await userEvent.type(screen.getByLabelText("Rate / 1k sqft"), "9");
		await userEvent.selectOptions(
			await screen.findByLabelText("Product"),
			"Milorganite",
		);
		expect(screen.getByLabelText("N %")).toHaveValue(6);
		expect(screen.getByLabelText("P %")).toHaveValue(4);
		expect(screen.getByLabelText("K %")).toHaveValue(0);
		expect(screen.getByLabelText("Rate / 1k sqft")).toHaveValue(32);

		await userEvent.clear(screen.getByLabelText("Rate / 1k sqft"));
		await userEvent.type(screen.getByLabelText("Rate / 1k sqft"), "25");
		await userEvent.clear(screen.getByLabelText("N %"));
		await userEvent.type(screen.getByLabelText("N %"), "5");
		expect(await save()).toMatchObject({
			product_name: "Milorganite",
			product_id: 5,
			rate_per_1000sqft: 25,
			nitrogen_pct: 5,
			phosphorus_pct: 4,
		});
	});

	// @spec APP-FORM-008, APP-FRAC-003
	it("ticks a picked fungicide's classes and keeps a typed rate without a label rate", async () => {
		api.listProducts.mockResolvedValue([
			product(6, {
				name: "Headway G",
				category: "Fungicide",
				frac_classes: ["Frac3", "Frac11"],
			}),
		]);
		await openForm();
		await userEvent.selectOptions(screen.getByLabelText("Type"), "Fungicide");
		await userEvent.type(screen.getByLabelText("Rate / 1k sqft"), "3");
		await userEvent.selectOptions(
			await screen.findByLabelText("Product"),
			"Headway G",
		);
		expect(await screen.findByLabelText("FRAC 3 — DMIs")).toBeChecked();
		expect(screen.getByLabelText("FRAC 11 — Strobilurins")).toBeChecked();
		expect(screen.getByLabelText("Rate / 1k sqft")).toHaveValue(3);

		// The name lookup (which knows no classes for it) runs, and changes nothing.
		await waitFor(() =>
			expect(api.getFracClasses).toHaveBeenCalledWith("Headway G"),
		);
		await act(async () => {
			await new Promise((resolve) => setTimeout(resolve, 50));
		});
		expect(screen.getByLabelText("FRAC 3 — DMIs")).toBeChecked();
		expect(screen.getByLabelText("FRAC 11 — Strobilurins")).toBeChecked();
	});

	// @spec APP-FORM-009
	it("offers to mark an Out product In stock after the save, and nothing else", async () => {
		const out = product(7, { name: "Dimension", stock_status: "Out" });
		const low = product(8, { name: "Lesco 24-0-11", stock_status: "Low" });
		api.listProducts.mockResolvedValue([out, low]);
		const confirm = vi.spyOn(window, "confirm").mockReturnValue(true);
		await openForm();

		await userEvent.selectOptions(
			await screen.findByLabelText("Product"),
			String(low.id),
		);
		await save();
		expect(confirm).not.toHaveBeenCalled();
		expect(api.markProductInStock).not.toHaveBeenCalled();

		await userEvent.click(
			await screen.findByRole("button", { name: "+ Add Application" }),
		);
		await userEvent.selectOptions(
			await screen.findByLabelText("Product"),
			String(out.id),
		);
		await save();
		await waitFor(() =>
			expect(confirm).toHaveBeenCalledWith(
				"Dimension is marked Out in your inventory. Mark it In stock?",
			),
		);
		expect(api.markProductInStock).toHaveBeenCalledTimes(1);
		expect(api.markProductInStock).toHaveBeenCalledWith(out);
	});
});
