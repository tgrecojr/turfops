// @vitest-environment jsdom
import { act, render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import { beforeEach, describe, expect, it, vi } from "vitest";
import * as client from "../api/client";
import { ApiError } from "../api/errors";
import type { DashboardResponse } from "../types";
import Dashboard from "./Dashboard";

vi.mock("../api/client", () => ({
	getDashboard: vi.fn(),
	getGdd: vi.fn(),
	getNitrogenBudget: vi.fn(),
	getSoilTempForecast: vi.fn(),
	getDiseaseRisk: vi.fn(),
	getTimingWindows: vi.fn(),
	getShoppingList: vi.fn(),
}));

const api = vi.mocked(client);

const DASHBOARD = {
	profile: { name: "Back Lawn", grass_type: "TallFescue", usda_zone: "7a" },
	environmental: {
		current: null,
		last_updated: null,
		soil_stale: false,
		soil_temp_7day_avg_f: null,
		ambient_temp_7day_avg_f: null,
		precipitation_7day_total_mm: null,
		soil_temp_trend: "Unknown",
	},
	recommendations: [],
	recent_applications: [],
	connections: { soildata: true, homeassistant: true, openweathermap: true },
} as unknown as DashboardResponse;

const FEATURES = [
	"getGdd",
	"getNitrogenBudget",
	"getSoilTempForecast",
	"getDiseaseRisk",
	"getTimingWindows",
] as const;

const outage = () =>
	new ApiError(
		503,
		"The weather data lake can't be read right now, so soil, disease, timing and history data are unavailable.",
	);

function renderDashboard() {
	return render(
		<MemoryRouter>
			<Dashboard />
		</MemoryRouter>,
	);
}

beforeEach(() => {
	vi.resetAllMocks();
	api.getShoppingList.mockResolvedValue({ generated_at: "", items: [] });
});

describe("dashboard load", () => {
	// @spec UI-DASH-001
	it("asks for the dashboard and every feature endpoint at once", async () => {
		let answer: (d: DashboardResponse) => void = () => {};
		api.getDashboard.mockReturnValue(
			new Promise((resolve) => {
				answer = resolve;
			}),
		);
		for (const name of FEATURES) api[name].mockRejectedValue(outage());
		renderDashboard();
		// Every request is out before the dashboard itself has answered.
		expect(api.getDashboard).toHaveBeenCalledTimes(1);
		for (const name of FEATURES)
			expect(api[name], name).toHaveBeenCalledTimes(1);

		await act(async () => answer(DASHBOARD));
		// Failing features are not a page error.
		expect(await screen.findByText("Back Lawn")).toBeInTheDocument();
		expect(screen.queryByRole("alert")).not.toBeInTheDocument();
	});

	// @spec UI-DASH-001
	it("shows a page error only when the dashboard request fails", async () => {
		api.getDashboard.mockRejectedValue(outage());
		for (const name of FEATURES) api[name].mockResolvedValue(null as never);
		renderDashboard();
		const alert = await screen.findByRole("alert");
		expect(alert).toHaveTextContent("weather data lake can't be read");
		expect(screen.queryByText("Back Lawn")).not.toBeInTheDocument();
	});

	// @spec UI-DASH-002
	it("keeps the last good dashboard on screen with the error beside it", async () => {
		api.getDashboard.mockResolvedValueOnce(DASHBOARD);
		for (const name of FEATURES) api[name].mockRejectedValue(outage());
		renderDashboard();
		expect(await screen.findByText("Back Lawn")).toBeInTheDocument();

		// The next poll (the tab coming back into view) fails.
		api.getDashboard.mockRejectedValueOnce(
			new ApiError(500, "Something went wrong on the server."),
		);
		await act(async () => {
			document.dispatchEvent(new Event("visibilitychange"));
		});
		const banner = await screen.findByRole("alert");
		expect(banner).toHaveTextContent(
			"Something went wrong on the server. Try again; the details are in the server log.",
		);
		expect(screen.getByText("Back Lawn")).toBeInTheDocument();
		expect(api.getDashboard).toHaveBeenCalledTimes(2);
	});
});
