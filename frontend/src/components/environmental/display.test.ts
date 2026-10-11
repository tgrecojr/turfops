import { describe, expect, it } from "vitest";
import type {
	DailyForecast,
	EnvironmentalSummary,
	ThresholdPrediction,
	TimeSeriesPoint,
} from "../../types";
import {
	crossingWhen,
	isPartialDay,
	lastDays,
	soilObservedLine,
	TREND_REFERENCES,
} from "./display";

describe("trend reference lines", () => {
	// @spec WX-VIEW-PAGE-007
	it("mark heat, humidity and drought only", () => {
		expect(TREND_REFERENCES).toEqual({
			ambient: { value: 85, label: "Heat stress" },
			humidity: { value: 80, label: "Humid (80%)" },
			moisture: { value: 0.1, label: "Drought" },
		});
	});
});

describe("forecast cards", () => {
	const day = (hours: number | undefined): DailyForecast =>
		({ date: "2026-10-11", hours_covered: hours }) as DailyForecast;

	// @spec WX-VIEW-PAGE-009
	it("call a day with fewer than 24 forecast hours partial", () => {
		expect(isPartialDay(day(9))).toBe(true);
		expect(isPartialDay(day(24))).toBe(false);
		expect(isPartialDay(day(undefined))).toBe(false);
	});
});

describe("station soil", () => {
	// @spec WX-VIEW-PAGE-008
	it("says when the station last observed the soil", () => {
		const summary = {
			soil_observed_at: "2026-10-10T18:00:00Z",
		} as EnvironmentalSummary;
		const line = soilObservedLine(summary);
		expect(line).toMatch(/^Station soil observed /);
		expect(soilObservedLine({} as EnvironmentalSummary)).toBeNull();
	});
});

describe("prediction chart", () => {
	// @spec WX-OUTLOOK-UI-004
	it("plots the last 7 days of history whatever the range", () => {
		const hourly: TimeSeriesPoint[] = Array.from(
			{ length: 30 * 4 },
			(_, i) => ({
				timestamp: new Date(
					Date.UTC(2026, 8, 11) + i * 6 * 3_600_000,
				).toISOString(),
				value: i,
			}),
		);
		const recent = lastDays(hourly, 7);
		const first = new Date(recent[0].timestamp).getTime();
		const last = new Date(recent[recent.length - 1].timestamp).getTime();
		expect(last - first).toBeLessThanOrEqual(7 * 86_400_000);
		expect(last - first).toBeGreaterThan(6.5 * 86_400_000);
	});
});

describe("crossings", () => {
	const crossing = (
		date: string | null,
		beyond: boolean,
	): ThresholdPrediction =>
		({
			threshold_name: "Grub Control Window",
			threshold_temp_f: 60,
			estimated_crossing_date: date,
			days_until_crossing: 12,
			beyond_forecast: beyond,
		}) as ThresholdPrediction;

	// @spec WX-OUTLOOK-CROSS-005
	it("never dates an estimate past the forecast", () => {
		expect(crossingWhen(crossing(null, true))).toBe("beyond the forecast");
		expect(crossingWhen(crossing("2026-10-14", false))).toBe(
			"~12 days (Oct 14)",
		);
	});
});

describe("gauges", () => {
	// @spec WX-VIEW-PAGE-010, WX-VIEW-PAGE-011
	it("warn on dry soil as well as wet, and say so in words", async () => {
		const { gaugeLevel, SOIL_MOISTURE_GAUGE, AMBIENT_TEMP_GAUGE } =
			await import("../gaugeConfigs");
		expect(gaugeLevel(SOIL_MOISTURE_GAUGE, 25)).toEqual({
			level: "ok",
			label: null,
		});
		expect(gaugeLevel(SOIL_MOISTURE_GAUGE, 14)).toEqual({
			level: "warning",
			label: "Dry",
		});
		expect(gaugeLevel(SOIL_MOISTURE_GAUGE, 9)).toEqual({
			level: "critical",
			label: "Very dry",
		});
		expect(gaugeLevel(SOIL_MOISTURE_GAUGE, 42)).toEqual({
			level: "warning",
			label: "High",
		});
		expect(gaugeLevel(AMBIENT_TEMP_GAUGE, 96)).toEqual({
			level: "critical",
			label: "Very high",
		});
		expect(gaugeLevel(AMBIENT_TEMP_GAUGE, null)).toEqual({
			level: "ok",
			label: null,
		});
	});
});
