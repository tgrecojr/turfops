// @vitest-environment jsdom
import { act, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
	getDashboard,
	getDiseaseRisk,
	getGdd,
	getNitrogenBudget,
	getSoilTempForecast,
	getTimingWindows,
} from "../api/client";
import type { DashboardResponse, Recommendation } from "../types";
import Dashboard from "./Dashboard";

vi.mock("../api/client", () => ({
	getDashboard: vi.fn(),
	getDiseaseRisk: vi.fn(),
	getGdd: vi.fn(),
	getNitrogenBudget: vi.fn(),
	getSoilTempForecast: vi.fn(),
	getTimingWindows: vi.fn(),
}));
// The widgets belong to other features; here they only mark where they render.
vi.mock("../components/Gauge", () => ({
	default: () => <div data-testid="gauge" />,
}));
vi.mock("../components/GddWidget", () => ({
	default: () => <div data-testid="widget">GDD widget</div>,
}));
vi.mock("../components/disease/DiseaseRiskWidget", () => ({
	default: () => <div data-testid="widget">Disease widget</div>,
}));
vi.mock("../components/timing/TimingWidget", () => ({
	default: () => <div data-testid="widget">Timing widget</div>,
}));
vi.mock("../components/inventory/ShoppingListPanel", () => ({
	default: () => <div data-testid="widget">Shopping widget</div>,
}));
vi.mock("../components/NitrogenBudgetWidget", () => ({
	default: () => <div data-testid="widget">Nitrogen widget</div>,
}));
vi.mock("../components/SoilTempForecastWidget", () => ({
	default: () => <div data-testid="widget">Soil forecast widget</div>,
}));

function alert(
	id: string,
	severity: Recommendation["severity"],
): Recommendation {
	return {
		id,
		category: "DiseasePressure",
		severity,
		title: `Alert ${id}`,
		description: `About ${id}`,
		explanation: "",
		data_points: [],
		suggested_action: null,
		needs: [],
		created_at: "2026-10-11T12:00:00Z",
		dismissed: false,
		addressed: false,
	};
}

const dashboard = {
	profile: { name: "Front Lawn", grass_type: "TallFescue", usda_zone: "7a" },
	environmental: {
		current: null,
		soil_temp_7day_avg_f: null,
		ambient_temp_7day_avg_f: null,
		humidity_7day_avg: null,
		precipitation_7day_total_mm: null,
		soil_temp_trend: "Unknown",
		last_updated: "2026-10-11T12:00:00Z",
		soil_observed_at: "2026-10-05T12:00:00Z",
		soil_stale: true,
		forecast: null,
		gdd_base50_ytd: null,
	},
	recommendations: [
		{
			...alert("brown_patch", "Critical"),
			inventory: { state: "NotOnHand", matches: [], missing: ["fungicide"] },
		},
		alert("dollar_spot", "Warning"),
		alert("aeration", "Advisory"),
	],
	recent_applications: [1, 2, 3, 4, 5].map((id) => ({
		id,
		application_type: "Mowing",
		product_name: `Entry ${id}`,
		application_date: `2026-10-0${id}`,
	})),
	connections: { soildata: true, homeassistant: false, openweathermap: true },
} as unknown as DashboardResponse;

const widgetFetchers = [
	getGdd,
	getNitrogenBudget,
	getSoilTempForecast,
	getDiseaseRisk,
	getTimingWindows,
].map((f) => vi.mocked(f as (...a: unknown[]) => Promise<unknown>));

let hidden = false;

beforeEach(() => {
	vi.mocked(getDashboard).mockReset().mockResolvedValue(dashboard);
	for (const f of widgetFetchers) f.mockReset().mockResolvedValue({});
	hidden = false;
	Object.defineProperty(document, "hidden", {
		configurable: true,
		get: () => hidden,
	});
});

afterEach(() => {
	vi.useRealTimers();
});

async function renderDashboard() {
	render(<Dashboard />);
	await screen.findByRole("heading", { name: "Front Lawn" });
}

function before(a: Element, b: Element) {
	return Boolean(
		a.compareDocumentPosition(b) & Node.DOCUMENT_POSITION_FOLLOWING,
	);
}

describe("Dashboard", () => {
	// @spec FEED-UI-008
	it("shows the endpoint's alerts as read-only cards", async () => {
		await renderDashboard();
		const column = screen.getByRole("heading", { name: "Active Alerts" })
			.parentElement as HTMLElement;
		const inColumn = within(column);
		expect(
			inColumn.getAllByText(/^Alert /).map((el) => el.textContent),
		).toEqual(["Alert brown_patch", "Alert dollar_spot", "Alert aeration"]);
		expect(inColumn.getByText("! Critical")).toBeInTheDocument();
		expect(inColumn.getAllByText("Disease Pressure")).toHaveLength(3);
		expect(inColumn.getByText("Need to buy")).toBeInTheDocument();
		expect(inColumn.getByText("About brown_patch")).toBeInTheDocument();
		expect(inColumn.queryAllByRole("button")).toHaveLength(0);
	});

	// @spec FEED-UI-009
	it("lays out header, stale banner, connections, gauges, widgets, then alerts beside recent applications", async () => {
		await renderDashboard();
		const header = screen.getByRole("heading", { name: "Front Lawn" });
		expect(header.parentElement).toHaveTextContent("Tall Fescue");
		expect(header.parentElement).toHaveTextContent("Zone 7a");
		expect(header.parentElement).toHaveTextContent("Updated");
		const banner = screen.getByRole("alert");
		expect(banner).toHaveTextContent("Station soil data stopped");
		const connections = screen.getByText("Weather lake");
		expect(connections.parentElement).toHaveTextContent("Home Assistant");
		expect(connections.parentElement).toHaveTextContent("OpenWeatherMap");
		const gauges = screen.getAllByTestId("gauge");
		expect(gauges).toHaveLength(4);
		const widgets = screen.getAllByTestId("widget");
		const alerts = screen.getByRole("heading", { name: "Active Alerts" });
		const recent = screen.getByRole("heading", { name: "Recent Applications" });
		const sequence = [
			header,
			banner,
			connections,
			gauges[0],
			widgets[0],
			alerts,
			recent,
		];
		for (let i = 1; i < sequence.length; i++) {
			expect(before(sequence[i - 1], sequence[i])).toBe(true);
		}
		const table = within(recent.parentElement as HTMLElement).getByRole(
			"table",
		);
		expect(within(table).getAllByRole("row")).toHaveLength(1 + 5);
	});

	// @spec FEED-UI-010
	it("polls every 30 s only while the tab is visible and re-fetches on return", async () => {
		vi.useFakeTimers();
		render(<Dashboard />);
		const flush = () => act(() => vi.advanceTimersByTimeAsync(0));
		const wait = (ms: number) => act(() => vi.advanceTimersByTimeAsync(ms));
		const calls = () => vi.mocked(getDashboard).mock.calls.length;
		await flush();
		expect(calls()).toBe(1);
		await wait(30_000);
		expect(calls()).toBe(2);

		hidden = true;
		document.dispatchEvent(new Event("visibilitychange"));
		await wait(90_000);
		expect(calls()).toBe(2);

		hidden = false;
		document.dispatchEvent(new Event("visibilitychange"));
		await flush();
		expect(calls()).toBe(3);
		await wait(30_000);
		expect(calls()).toBe(4);
	});

	// @spec FEED-UI-011
	it("hides a widget whose first fetch fails and renders the rest", async () => {
		vi.mocked(getGdd).mockRejectedValue(new Error("lake down"));
		await renderDashboard();
		expect(screen.queryByText("GDD widget")).not.toBeInTheDocument();
		for (const name of ["Disease widget", "Timing widget", "Nitrogen widget"]) {
			expect(screen.getByText(name)).toBeInTheDocument();
		}
		expect(screen.getByText("Alert brown_patch")).toBeInTheDocument();
		expect(screen.getAllByTestId("gauge")).toHaveLength(4);
	});
});
