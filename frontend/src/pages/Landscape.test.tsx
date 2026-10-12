// @vitest-environment jsdom
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import * as client from "../api/client";
import { ApiError } from "../api/errors";
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

function plant(id: number, name: string, extra: Partial<Plant> = {}): Plant {
	return {
		id,
		lawn_profile_id: 1,
		common_name: name,
		scientific_name: "Ilex opaca",
		plant_type: "Shrub",
		location: "Front walk",
		planting_date: "2024-04-01",
		notes: null,
		maintenance_plan: {
			identified_name: name,
			scientific_name: "Ilex opaca",
			identification_confidence: "High",
			summary: `${name} summary`,
			tasks: [
				{
					task_type: "PestInspection",
					window_start_month_day: "03-15",
					window_end_month_day: "04-15",
					frequency: "Twice",
					description: "Check for scale.",
					severity: "Info",
					zone_note: "Later in zone 6",
				},
			],
			warnings: ["Berries are toxic to pets"],
		},
		plan_generated_at: "2026-03-01T12:00:00Z",
		plan_model: "test/plan-model",
		created_at: "2026-03-01T12:00:00Z",
		updated_at: "2026-03-01T12:00:00Z",
		...extra,
	};
}

beforeEach(() => {
	vi.resetAllMocks();
});

afterEach(() => {
	vi.restoreAllMocks();
});

const row = (name: string) =>
	screen.getByRole("button", { name: new RegExp(`^. ${name}`) });

// @spec PLANT-UI-002
describe("Landscape header", () => {
	it("toggles the add form and says where the windows appear", async () => {
		api.listPlants.mockResolvedValue([]);
		render(<Landscape />);
		expect(
			screen.getByText(/Calendar,\s+Seasonal Plan, and Recommendations/),
		).toBeInTheDocument();
		const add = screen.getByRole("button", { name: "+ Add Plant" });
		await userEvent.click(add);
		expect(
			screen.getByLabelText(/Common name or scientific name/),
		).toBeVisible();
		await userEvent.click(screen.getByRole("button", { name: "Cancel" }));
		expect(
			screen.queryByLabelText(/Common name or scientific name/),
		).toBeNull();
	});
});

// @spec PLANT-UI-006
describe("Landscape list", () => {
	it("sorts by name ignoring case and shows each collapsed row's summary", async () => {
		api.listPlants.mockResolvedValue([plant(1, "boxwood"), plant(2, "Azalea")]);
		render(<Landscape />);
		const rows = await screen.findAllByRole("button", { expanded: false });
		expect(rows.map((r) => r.textContent)).toEqual([
			expect.stringContaining("Azalea"),
			expect.stringContaining("boxwood"),
		]);
		const azalea = within(rows[0]);
		expect(azalea.getByText("Ilex opaca")).toBeInTheDocument();
		expect(azalea.getByText("Shrub")).toBeInTheDocument();
		expect(azalea.getByText(/Front walk/)).toBeInTheDocument();
		expect(azalea.getByText("1 tasks")).toBeInTheDocument();
		expect(azalea.getByText("Azalea summary")).toBeInTheDocument();
		expect(azalea.getByText("High confidence")).toBeInTheDocument();
	});
});

// @spec PLANT-UI-007
describe("Landscape expanded row", () => {
	it("shows the plan, its tasks and the footer actions", async () => {
		api.listPlants.mockResolvedValue([plant(1, "Holly")]);
		render(<Landscape />);
		await userEvent.click(
			await screen.findByRole("button", { expanded: false }),
		);
		expect(row("Holly")).toHaveAttribute("aria-expanded", "true");
		expect(screen.getByText("Holly summary")).toBeInTheDocument();
		expect(screen.getByText("Planted 2024-04-01")).toBeInTheDocument();
		expect(screen.getByText(/Berries are toxic to pets/)).toBeInTheDocument();
		expect(screen.getByText("Pest Inspection")).toBeInTheDocument();
		expect(screen.getByText("Mar 15 – Apr 15")).toBeInTheDocument();
		expect(screen.getByText("Twice")).toBeInTheDocument();
		expect(screen.getByText("Check for scale.")).toBeInTheDocument();
		expect(screen.getByText("Later in zone 6")).toBeInTheDocument();
		expect(
			screen.getByText(/Plan generated .* test\/plan-model/),
		).toBeVisible();
		expect(
			screen.getByRole("button", { name: "Regenerate plan" }),
		).toBeInTheDocument();
		expect(screen.getByRole("button", { name: "Delete" })).toBeInTheDocument();
	});
});

// @spec PLANT-UI-008
describe("Landscape delete", () => {
	it("asks before deleting and removes the row once confirmed", async () => {
		api.listPlants.mockResolvedValue([plant(1, "Holly")]);
		api.deletePlant.mockResolvedValue(undefined);
		const confirm = vi.spyOn(window, "confirm").mockReturnValue(false);
		render(<Landscape />);
		await userEvent.click(
			await screen.findByRole("button", { expanded: false }),
		);
		await userEvent.click(screen.getByRole("button", { name: "Delete" }));
		expect(confirm).toHaveBeenCalledWith(expect.stringContaining("Holly"));
		expect(api.deletePlant).not.toHaveBeenCalled();

		confirm.mockReturnValue(true);
		await userEvent.click(screen.getByRole("button", { name: "Delete" }));
		expect(api.deletePlant).toHaveBeenCalledWith(1);
		expect(await screen.findByText(/No plants yet/)).toBeInTheDocument();
	});
});

// @spec PLANT-UI-009
describe("Landscape empty", () => {
	it("invites the owner to add a plant", async () => {
		api.listPlants.mockResolvedValue([]);
		render(<Landscape />);
		expect(
			await screen.findByText(
				'No plants yet. Click "+ Add Plant" to get started.',
			),
		).toBeInTheDocument();
	});
});

// @spec PLANT-UI-010
describe("Landscape errors", () => {
	it("shows the server's sentence for a 503, never the status or JSON", async () => {
		const sentence =
			"The plan assistant couldn't answer just now. Try again in a few minutes.";
		api.listPlants.mockRejectedValue(new ApiError(503, sentence));
		render(<Landscape />);
		expect(await screen.findByText(sentence)).toBeInTheDocument();
		expect(document.body.textContent).not.toMatch(/503|\{/);
	});

	it("says a regeneration that timed out took too long", async () => {
		api.listPlants.mockResolvedValue([plant(1, "Holly")]);
		api.refreshPlantPlan.mockRejectedValue(
			new ApiError(0, "The request timed out.", true),
		);
		render(<Landscape />);
		await userEvent.click(
			await screen.findByRole("button", { expanded: false }),
		);
		await userEvent.click(
			screen.getByRole("button", { name: "Regenerate plan" }),
		);
		expect(await screen.findByText(/took too long/)).toHaveTextContent(
			"Try again",
		);
	});
});

// @spec PLANT-UI-011
describe("Landscape without a lawn profile", () => {
	it("sends the owner to Settings", async () => {
		api.listPlants.mockRejectedValue(
			new ApiError(404, "No lawn profile found"),
		);
		render(<Landscape />);
		expect(await screen.findByText(/Settings/)).toHaveTextContent(
			/lawn profile/,
		);
	});
});
