// @vitest-environment jsdom
import { act, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
	getEnvironmental,
	getHistorical,
	getSoilTempForecast,
	refreshEnvironmental,
} from "../api/client";
import {
	AMBIENT_TEMP_GAUGE,
	HUMIDITY_GAUGE,
	SOIL_MOISTURE_GAUGE,
	SOIL_TEMP_GAUGE,
} from "../components/gaugeConfigs";
import {
	historical,
	soilForecast,
	summary,
} from "../test/environmentalFixtures";
import Environmental from "./Environmental";

vi.mock("../api/client", () => ({
	getEnvironmental: vi.fn(),
	getHistorical: vi.fn(),
	getSoilTempForecast: vi.fn(),
	refreshEnvironmental: vi.fn(),
}));

// jsdom has no layout; the charts' own behaviour is tested in PredictionChart.test.tsx.
vi.mock("recharts", () => {
	const none = () => null;
	return {
		Area: none,
		AreaChart: none,
		CartesianGrid: none,
		ComposedChart: none,
		Line: none,
		ReferenceLine: none,
		ResponsiveContainer: none,
		Tooltip: none,
		XAxis: none,
		YAxis: none,
	};
});

let hidden = false;

beforeEach(() => {
	hidden = false;
	Object.defineProperty(document, "hidden", {
		configurable: true,
		get: () => hidden,
	});
	vi.mocked(getEnvironmental).mockResolvedValue(summary());
	vi.mocked(getHistorical).mockResolvedValue(historical());
	vi.mocked(getSoilTempForecast).mockResolvedValue(soilForecast());
});

afterEach(() => {
	vi.useRealTimers();
	vi.clearAllMocks();
});

/** A gauge's header text (its label is a span; a trend chart's is a div). */
function gaugeText(label: string): string {
	const gauge = screen.getAllByText(label).find((el) => el.tagName === "SPAN");
	return gauge?.parentElement?.textContent ?? "";
}

describe("Environmental page loading", () => {
	// @spec WX-VIEW-PAGE-001
	it("loads conditions and the soil outlook together and polls every 30 s only while visible", async () => {
		vi.useFakeTimers({ shouldAdvanceTime: true });
		render(<Environmental />);
		await screen.findByText("Soil Profile by Depth");
		expect(getEnvironmental).toHaveBeenCalledTimes(1);
		expect(getSoilTempForecast).toHaveBeenCalledTimes(1);

		await act(() => vi.advanceTimersByTimeAsync(30_000));
		expect(getEnvironmental).toHaveBeenCalledTimes(2);
		expect(getSoilTempForecast).toHaveBeenCalledTimes(2);

		hidden = true;
		act(() => {
			document.dispatchEvent(new Event("visibilitychange"));
		});
		await act(() => vi.advanceTimersByTimeAsync(120_000));
		expect(getEnvironmental).toHaveBeenCalledTimes(2);

		hidden = false;
		await act(async () => {
			document.dispatchEvent(new Event("visibilitychange"));
		});
		expect(getEnvironmental).toHaveBeenCalledTimes(3);
		await act(() => vi.advanceTimersByTimeAsync(30_000));
		expect(getEnvironmental).toHaveBeenCalledTimes(4);
	});

	// @spec WX-VIEW-PAGE-002
	it("Refresh Now forces a refresh and shows its result", async () => {
		const fresh = summary();
		if (fresh.current) fresh.current.soil_temp_10_f = 70.3;
		vi.mocked(refreshEnvironmental).mockResolvedValue(fresh);
		render(<Environmental />);
		await userEvent.click(
			await screen.findByRole("button", { name: "Refresh Now" }),
		);
		expect(refreshEnvironmental).toHaveBeenCalledTimes(1);
		expect(gaugeText(SOIL_TEMP_GAUGE.label)).toContain("70.3");
	});

	// @spec WX-OUTLOOK-UI-003
	it("hides the soil outlook section when the outlook is unavailable", async () => {
		vi.mocked(getSoilTempForecast).mockRejectedValue(new Error("503"));
		render(<Environmental />);
		await screen.findByText("Soil Profile by Depth");
		expect(
			screen.queryByText("Soil Temperature Forecast"),
		).not.toBeInTheDocument();
		expect(gaugeText(SOIL_TEMP_GAUGE.label)).toContain("62.4");
	});
});

describe("Environmental page content", () => {
	// @spec WX-VIEW-PAGE-003
	it("shows the header, four gauges, 7-day cards, soil profile and forecast cards", async () => {
		render(<Environmental />);
		expect(await screen.findByText(/Last updated:/)).toHaveTextContent(
			"Soil temp trend: Rising",
		);

		expect(gaugeText(SOIL_TEMP_GAUGE.label)).toContain("62.4 °F");
		expect(gaugeText(AMBIENT_TEMP_GAUGE.label)).toContain("71.5 °F");
		expect(gaugeText(HUMIDITY_GAUGE.label)).toContain("55.5 %");
		expect(gaugeText(SOIL_MOISTURE_GAUGE.label)).toContain("25.0 %");
		const ranges = [
			SOIL_TEMP_GAUGE,
			AMBIENT_TEMP_GAUGE,
			HUMIDITY_GAUGE,
			SOIL_MOISTURE_GAUGE,
		].map((g) => [g.min, g.max, g.thresholds.warn, g.thresholds.critical]);
		expect(ranges).toEqual([
			[30, 100, 75, 85],
			[0, 110, 85, 95],
			[0, 100, 80, 90],
			[0, 50, 40, 45],
		]);

		for (const card of [
			"7-Day Soil Temp Avg",
			"7-Day Ambient Avg",
			"7-Day Humidity Avg",
			"7-Day Precipitation",
		]) {
			expect(screen.getByText(card)).toBeInTheDocument();
		}

		const profile = screen.getByText("Soil Profile by Depth")
			.nextElementSibling as HTMLElement;
		const depths = within(profile)
			.getAllByRole("row")
			.slice(1)
			.map((row) => row.firstElementChild?.textContent);
		expect(depths).toEqual(["5 cm", "10 cm", "20 cm", "50 cm", "100 cm"]);
		expect(within(profile).getByText("61.2 °F")).toBeInTheDocument();

		const cards = screen.getByText("5-Day Forecast")
			.nextElementSibling as HTMLElement;
		expect(cards.children).toHaveLength(2);
		const first = cards.children[0] as HTMLElement;
		expect(first).toHaveTextContent("2026-10-12");
		expect(first).toHaveTextContent("68° / 48°");
		expect(first).toHaveTextContent("Humidity: 64%");
		expect(first).toHaveTextContent(/Precip: 1\.00 in \(40%\)/);
	});

	// @spec WX-VIEW-PAGE-003
	it("shows -- for a gauge with no reading", async () => {
		const data = summary();
		if (data.current) data.current.ambient_temp_f = null;
		vi.mocked(getEnvironmental).mockResolvedValue(data);
		render(<Environmental />);
		await screen.findByText("Soil Profile by Depth");
		expect(gaugeText(AMBIENT_TEMP_GAUGE.label)).toContain("--");
	});

	// @spec WX-VIEW-PAGE-004
	it("shows precipitation in inches converted from the API's millimetres", async () => {
		render(<Environmental />);
		const card = (await screen.findByText("7-Day Precipitation"))
			.parentElement as HTMLElement;
		expect(card).toHaveTextContent("1.00 in");
		const cards = screen.getByText("5-Day Forecast")
			.nextElementSibling as HTMLElement;
		expect(cards.children[0]).toHaveTextContent("Precip: 1.00 in");
		expect(cards.children[1]).toHaveTextContent("Precip: 0.00 in");
	});

	// @spec WX-OUTLOOK-UI-001
	it("shows crossing cards and the model line under the prediction chart", async () => {
		render(<Environmental />);
		const name = await screen.findByText("Active Growth Peak");
		const card = name.parentElement?.parentElement as HTMLElement;
		expect(within(card).getByText("Medium")).toBeInTheDocument();
		expect(card).toHaveTextContent("↓ 65°F");
		expect(card).toHaveTextContent("~3 days (Oct 14)");
		expect(screen.getByText(/^Model:/)).toHaveTextContent(
			/=0\.82, lag=2d, quality=Good/,
		);
	});
});
