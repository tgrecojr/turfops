// @vitest-environment jsdom
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import * as client from "../api/client";
import type { Plant } from "../types";
import Landscape from "./Landscape";

vi.mock("../api/client", () => ({
	listPlants: vi.fn(),
	createPlant: vi.fn(),
	deletePlant: vi.fn(),
	refreshPlantPlan: vi.fn(),
	updatePlant: vi.fn(),
}));

const api = vi.mocked(client);

const created: Plant = {
	id: 9,
	lawn_profile_id: 1,
	common_name: "Panicle hydrangea",
	scientific_name: "Hydrangea paniculata",
	plant_type: "Shrub",
	location: null,
	planting_date: null,
	notes: null,
	maintenance_plan: {
		identified_name: "Panicle hydrangea",
		scientific_name: "Hydrangea paniculata",
		identification_confidence: "High",
		summary: "A hardy flowering shrub.",
		tasks: [],
		warnings: [],
	},
	plan_generated_at: "2026-03-01T12:00:00Z",
	plan_model: "m",
	created_at: "2026-03-01T12:00:00Z",
	updated_at: "2026-03-01T12:00:00Z",
};

beforeEach(() => {
	vi.resetAllMocks();
	api.listPlants.mockResolvedValue([]);
});

async function openForm() {
	render(<Landscape />);
	await screen.findByText(/No plants yet/);
	await userEvent.click(screen.getByRole("button", { name: "+ Add Plant" }));
}

async function submit(name: string) {
	await userEvent.type(
		screen.getByLabelText(/Common name or scientific name/),
		name,
	);
	await userEvent.click(
		screen.getByRole("button", { name: "Add plant & generate plan" }),
	);
}

// @spec PLANT-UI-003
describe("Landscape add form", () => {
	it("offers one name field, a Shrub default and the optional details", async () => {
		await openForm();
		expect(screen.getByLabelText("Plant type")).toHaveValue("Shrub");
		expect(screen.getByLabelText("Location (optional)")).toBeInTheDocument();
		expect(
			screen.getByLabelText("Planting date (optional)"),
		).toBeInTheDocument();
		expect(screen.getByLabelText("Notes (optional)")).toBeInTheDocument();
	});

	it("sends a Genus species name as the scientific name", async () => {
		api.createPlant.mockResolvedValue(created);
		await openForm();
		await submit("Hydrangea paniculata");
		expect(api.createPlant).toHaveBeenCalledWith(
			expect.objectContaining({
				common_name: undefined,
				scientific_name: "Hydrangea paniculata",
				plant_type: "Shrub",
			}),
		);
	});

	it("sends anything else as the common name", async () => {
		api.createPlant.mockResolvedValue(created);
		await openForm();
		await submit("panicle hydrangea");
		expect(api.createPlant).toHaveBeenCalledWith(
			expect.objectContaining({
				common_name: "panicle hydrangea",
				scientific_name: undefined,
			}),
		);
	});
});

// @spec PLANT-UI-004
describe("Landscape add in flight", () => {
	it("says the plan is generating and may take 10–30 seconds", async () => {
		let finish: (p: Plant) => void = () => {};
		api.createPlant.mockReturnValue(
			new Promise<Plant>((resolve) => {
				finish = resolve;
			}),
		);
		await openForm();
		expect(screen.getByText(/may take 10–30 seconds/)).toBeInTheDocument();
		await submit("Holly");
		expect(
			screen.getByRole("button", { name: "Generating plan…" }),
		).toBeDisabled();
		finish(created);
		expect(await screen.findByText("Panicle hydrangea")).toBeInTheDocument();
	});
});

// @spec PLANT-UI-005
describe("Landscape after adding", () => {
	it("closes the form and shows the new plant expanded", async () => {
		api.createPlant.mockResolvedValue(created);
		await openForm();
		await submit("Hydrangea paniculata");
		const row = await screen.findByRole("button", { expanded: true });
		expect(row).toHaveTextContent("Panicle hydrangea");
		expect(
			screen.queryByLabelText(/Common name or scientific name/),
		).toBeNull();
		expect(
			screen.getByRole("button", { name: "+ Add Plant" }),
		).toBeInTheDocument();
	});
});
