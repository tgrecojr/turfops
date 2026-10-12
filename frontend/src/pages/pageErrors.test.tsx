// @vitest-environment jsdom
import { render, screen } from "@testing-library/react";
import type { ComponentType } from "react";
import { MemoryRouter, Route, Routes } from "react-router";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import Applications from "./Applications";
import Calendar from "./Calendar";
import Dashboard from "./Dashboard";
import DiseaseRisk from "./DiseaseRisk";
import Environmental from "./Environmental";
import Inventory from "./Inventory";
import Landscape from "./Landscape";
import Recommendations from "./Recommendations";
import SeasonalPlan from "./SeasonalPlan";
import Settings from "./Settings";
import SoilTests from "./SoilTests";
import Timing from "./Timing";

const OUTAGE = "Test outage: that data source can't answer right now.";

// Every request the client makes fails, the way a lost data source does.
vi.mock("../api/client", async (importOriginal) => {
	const real = await importOriginal<Record<string, unknown>>();
	const { ApiError: Err } = await import("../api/errors");
	return Object.fromEntries(
		Object.entries(real).map(([name, value]) => [
			name,
			typeof value === "function"
				? () =>
						Promise.reject(
							new Err(
								503,
								"Test outage: that data source can't answer right now.",
							),
						)
				: value,
		]),
	);
});

const PAGES: [string, string, string, ComponentType][] = [
	["Dashboard", "/", "/", Dashboard],
	["Applications", "/applications", "/applications", Applications],
	["Inventory", "/inventory", "/inventory", Inventory],
	["Landscape", "/landscape", "/landscape", Landscape],
	["Calendar", "/calendar", "/calendar", Calendar],
	["Environmental", "/environmental", "/environmental", Environmental],
	["Recommendations", "/recommendations", "/recommendations", Recommendations],
	["Disease Risk", "/disease-risk", "/disease-risk", DiseaseRisk],
	[
		"Disease detail",
		"/disease-risk/:slug",
		"/disease-risk/brown-patch",
		DiseaseRisk,
	],
	["Timing", "/timing", "/timing", Timing],
	["Soil Tests", "/soil-tests", "/soil-tests", SoilTests],
	["Seasonal Plan", "/seasonal-plan", "/seasonal-plan", SeasonalPlan],
	["Settings", "/settings", "/settings", Settings],
];

let consoleError: ReturnType<typeof vi.spyOn>;
beforeEach(() => {
	consoleError = vi.spyOn(console, "error").mockImplementation(() => {});
});
afterEach(() => {
	consoleError.mockRestore();
});

describe("a failed request", () => {
	// @spec UI-SHELL-005
	it.each(PAGES)(
		"is shown inline on the %s page",
		async (_name, pattern, path, Page) => {
			render(
				<MemoryRouter initialEntries={[path]}>
					<Routes>
						<Route path={pattern} element={<Page />} />
					</Routes>
				</MemoryRouter>,
			);
			const shown = await screen.findAllByText(OUTAGE, { exact: false });
			expect(shown.length).toBeGreaterThan(0);
			expect(
				screen.queryByText("Something went wrong"),
			).not.toBeInTheDocument();
			expect(
				consoleError.mock.calls.some((call: unknown[]) =>
					/uncaught|error boundary/i.test(String(call[0])),
				),
			).toBe(false);
		},
	);
});
