// @vitest-environment jsdom
import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
	getDashboard,
	getDiseaseRisk,
	getGdd,
	getNitrogenBudget,
	getShoppingList,
	getSoilTempForecast,
	getTimingWindows,
} from "../api/client";
import { soilForecast, summary } from "../test/environmentalFixtures";
import type { DashboardResponse } from "../types";
import Dashboard from "./Dashboard";

vi.mock("../api/client", () => ({
	getDashboard: vi.fn(),
	getDiseaseRisk: vi.fn(),
	getGdd: vi.fn(),
	getNitrogenBudget: vi.fn(),
	getShoppingList: vi.fn(),
	getSoilTempForecast: vi.fn(),
	getTimingWindows: vi.fn(),
}));

const dashboard: DashboardResponse = {
	profile: {
		id: 1,
		name: "Front Lawn",
		grass_type: "TallFescue",
		usda_zone: "7a",
		soil_type: null,
		lawn_size_sqft: null,
		irrigation_type: null,
		created_at: "2026-01-01T00:00:00Z",
		updated_at: "2026-01-01T00:00:00Z",
	},
	environmental: summary(),
	recommendations: [],
	recent_applications: [],
	connections: { soildata: true, homeassistant: true, openweathermap: true },
};

beforeEach(() => {
	vi.mocked(getDashboard).mockResolvedValue(dashboard);
	vi.mocked(getShoppingList).mockResolvedValue({ items: [] } as never);
	// The other widgets are out of scope here: their loads fail, so they don't render.
	for (const other of [
		getDiseaseRisk,
		getGdd,
		getNitrogenBudget,
		getTimingWindows,
	]) {
		vi.mocked(other).mockRejectedValue(new Error("unavailable"));
	}
});

afterEach(() => {
	vi.clearAllMocks();
});

function renderDashboard() {
	render(
		<MemoryRouter>
			<Dashboard />
		</MemoryRouter>,
	);
	return screen.findByText("Front Lawn");
}

describe("Dashboard soil outlook widget", () => {
	// @spec WX-OUTLOOK-UI-003
	it("is not rendered when the soil outlook is unavailable", async () => {
		vi.mocked(getSoilTempForecast).mockRejectedValue(new Error("503"));
		await renderDashboard();
		expect(getSoilTempForecast).toHaveBeenCalled();
		expect(screen.queryByText("Soil Temp Forecast")).not.toBeInTheDocument();
	});

	// @spec WX-OUTLOOK-UI-003
	it("is rendered when the soil outlook loads", async () => {
		vi.mocked(getSoilTempForecast).mockResolvedValue(soilForecast());
		await renderDashboard();
		expect(await screen.findByText("Soil Temp Forecast")).toBeInTheDocument();
	});
});
