// @vitest-environment jsdom
import { fireEvent, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import * as client from "../api/client";
import { ApiError } from "../api/errors";
import { soilTest, summary } from "../test/soilTestFixtures";
import { todayLocalISO } from "../utils/dates";
import SoilTests from "./SoilTests";

vi.mock("../api/client", async (importOriginal) => ({
	...(await importOriginal<typeof import("../api/client")>()),
	getSoilTests: vi.fn(),
	getSoilTestRecommendations: vi.fn(),
	createSoilTest: vi.fn(),
	updateSoilTest: vi.fn(),
	deleteSoilTest: vi.fn(),
	getToday: vi.fn(),
}));

const api = vi.mocked(client);
const newer = soilTest({
	id: 2,
	test_date: "2026-03-23",
	lab_name: "Waypoint",
});
const older = soilTest({
	id: 1,
	test_date: "2023-04-02",
	lab_name: null,
	ph: 5.8,
	phosphorus_ppm: null,
});

beforeEach(() => {
	api.getToday.mockResolvedValue({
		today: "2026-10-11",
		timezone: "America/New_York",
	});
	api.getSoilTests.mockResolvedValue([newer, older]);
	api.getSoilTestRecommendations.mockResolvedValue(summary());
	api.createSoilTest.mockResolvedValue(newer);
	api.updateSoilTest.mockResolvedValue(newer);
	api.deleteSoilTest.mockResolvedValue(undefined);
});

afterEach(() => {
	vi.clearAllMocks();
	vi.unstubAllGlobals();
});

async function loaded() {
	render(<SoilTests />);
	await screen.findByRole("heading", { name: "Test History" });
}

async function openForm() {
	await userEvent.click(
		screen.getByRole("button", { name: "+ Add Soil Test" }),
	);
}

const phInput = () => screen.getByLabelText(/^pH/) as HTMLInputElement;

describe("Soil Tests page", () => {
	// @spec SOIL-UI-002
	it("lists the tests and asks for advice only when one exists", async () => {
		await loaded();
		expect(api.getSoilTests).toHaveBeenCalledTimes(1);
		expect(api.getSoilTestRecommendations).toHaveBeenCalledTimes(1);
		expect(
			await screen.findByRole("heading", { name: "Recommendations" }),
		).toBeInTheDocument();
	});

	// @spec SOIL-UI-002, SOIL-UI-011
	it("with no test stored says so and shows no advice", async () => {
		api.getSoilTests.mockResolvedValue([]);
		await loaded();
		expect(
			screen.getByText(
				"No soil tests recorded yet. Add your first test above.",
			),
		).toBeInTheDocument();
		expect(api.getSoilTestRecommendations).not.toHaveBeenCalled();
		expect(
			screen.queryByRole("heading", { name: "Recommendations" }),
		).not.toBeInTheDocument();
	});

	// @spec SOIL-UI-010
	it("lists tests newest first with their key values, Edit and Delete", async () => {
		await loaded();
		const history = screen.getByRole("heading", { name: "Test History" });
		const table = within(history.nextElementSibling as HTMLElement).getByRole(
			"table",
		);
		const headers = within(table)
			.getAllByRole("columnheader")
			.map((h) => h.textContent);
		expect(headers.slice(0, 8)).toEqual([
			"Date",
			"Lab",
			"pH",
			"P (ppm)",
			"K (ppm)",
			"Ca (ppm)",
			"Mg (ppm)",
			"OM %",
		]);
		const rows = within(table).getAllByRole("row").slice(1);
		const first = within(rows[0]).getAllByRole("cell");
		expect(first.slice(0, 8).map((c) => c.textContent)).toEqual([
			"2026-03-23",
			"Waypoint",
			"6.0",
			"10",
			"80",
			"300",
			"44",
			"3.2",
		]);
		expect(within(rows[1]).getAllByRole("cell")[0]).toHaveTextContent(
			"2023-04-02",
		);

		await userEvent.click(
			within(rows[1]).getByRole("button", { name: "Edit" }),
		);
		expect(
			screen.getByRole("heading", { name: "Edit Soil Test" }),
		).toBeInTheDocument();
		expect(phInput().value).toBe("5.8");

		vi.stubGlobal("confirm", () => true);
		await userEvent.click(
			within(rows[0]).getByRole("button", { name: "Delete" }),
		);
		expect(api.deleteSoilTest).toHaveBeenCalledWith(2);
	});

	// @spec SOIL-UI-003
	it("adds a test from one grouped form dated today, sending empty fields as absent", async () => {
		await loaded();
		await openForm();
		for (const group of [
			"Core",
			"Macronutrients (ppm)",
			"Micronutrients (ppm)",
			"Other",
		]) {
			expect(screen.getByRole("heading", { name: group })).toBeInTheDocument();
		}
		const date = screen.getByLabelText(/^Date/) as HTMLInputElement;
		expect(date.value).toBe(todayLocalISO());
		await userEvent.type(phInput(), "6.2");
		await userEvent.click(
			screen.getByRole("button", { name: "Save Soil Test" }),
		);

		expect(api.createSoilTest).toHaveBeenCalledTimes(1);
		const sent = api.createSoilTest.mock.calls[0][0];
		expect(sent.test_date).toBe(todayLocalISO());
		expect(sent.ph).toBe(6.2);
		expect(sent.phosphorus_ppm).toBeUndefined();
		expect(sent.copper_ppm).toBeUndefined();
		expect(sent.lab_name).toBeUndefined();
	});

	// @spec SOIL-UI-004
	it("refuses to save without a pH", async () => {
		await loaded();
		await openForm();
		const form = phInput().closest("form");
		if (!form) throw new Error("no form");
		fireEvent.submit(form);
		expect(await screen.findByText("pH is required")).toBeInTheDocument();
		expect(api.createSoilTest).not.toHaveBeenCalled();
		expect(api.updateSoilTest).not.toHaveBeenCalled();
	});

	// @spec SOIL-UI-005
	it("after a save closes and resets the form and reloads tests and advice", async () => {
		await loaded();
		await userEvent.click(screen.getAllByRole("button", { name: "Edit" })[0]);
		await userEvent.click(
			screen.getByRole("button", { name: "Update Soil Test" }),
		);
		expect(api.updateSoilTest).toHaveBeenCalledWith(2, expect.anything());
		expect(
			screen.queryByRole("heading", { name: "Edit Soil Test" }),
		).not.toBeInTheDocument();
		expect(api.getSoilTests).toHaveBeenCalledTimes(2);
		expect(api.getSoilTestRecommendations).toHaveBeenCalledTimes(2);

		await openForm();
		expect(
			screen.getByRole("heading", { name: "New Soil Test" }),
		).toBeInTheDocument();
		expect(phInput().value).toBe("");
	});

	// @spec SOIL-UI-013
	it.each([
		[
			new ApiError(404, "No lawn profile found"),
			"There is no lawn profile yet. Set one up in Settings to get started.",
		],
		[new ApiError(0, "", true), "That took too long to answer. Try again."],
		[
			new ApiError(400, "pH must be between 0 and 14"),
			"pH must be between 0 and 14",
		],
	])(
		"explains a failed request in plain words (%s)",
		async (error, sentence) => {
			api.getSoilTests.mockRejectedValue(error);
			render(<SoilTests />);
			expect(await screen.findByText(sentence)).toBeInTheDocument();
			expect(document.body).not.toHaveTextContent(/\b(404|400)\b/);
		},
	);

	// @spec SOIL-UI-013
	it("explains a failed save with the server's message", async () => {
		api.createSoilTest.mockRejectedValue(
			new ApiError(
				400,
				"Phosphorus can't be negative. Check the value on your lab report.",
			),
		);
		await loaded();
		await openForm();
		await userEvent.type(phInput(), "6.2");
		await userEvent.click(
			screen.getByRole("button", { name: "Save Soil Test" }),
		);
		expect(
			await screen.findByText(
				"Phosphorus can't be negative. Check the value on your lab report.",
			),
		).toBeInTheDocument();
	});
});
