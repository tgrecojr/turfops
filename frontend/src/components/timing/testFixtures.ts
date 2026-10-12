// Shared builders for the timing component tests.
import type {
	BoundaryView,
	DateStat,
	SoilSeriesDay,
	TimingResponse,
	TimingWindow,
	WindowId,
	WindowPriority,
	WindowState,
} from "../../types/timing";

export function stat(
	median: string,
	overrides: Partial<DateStat> = {},
): DateStat {
	return {
		median,
		p10: median,
		p90: median,
		earliest: median,
		latest: median,
		sample_count: 10,
		confidence: "High",
		...overrides,
	};
}

export function boundary(overrides: Partial<BoundaryView> = {}): BoundaryView {
	return {
		label: "5 cm soil cools to 70°F",
		typical: null,
		date: null,
		source: "Typical",
		passed: false,
		days_held: null,
		...overrides,
	};
}

export function timingWindow(
	id: WindowId,
	state: WindowState,
	overrides: Partial<TimingWindow> = {},
): TimingWindow {
	const priority: WindowPriority =
		id === "spring_seeding"
			? "Secondary"
			: id === "dormant_seeding"
				? "Optional"
				: "Primary";
	return {
		id,
		name: `Window ${id}`,
		priority,
		season_year: 2026,
		state,
		headline: `Headline ${id}`,
		detail: `Detail ${id}`,
		opens: boundary({
			typical: stat("2026-09-07", {
				earliest: "2026-08-25",
				latest: "2026-09-20",
			}),
		}),
		ideal_from: null,
		ideal_until: null,
		closes: boundary({
			label: "5 cm soil cools to 55°F",
			typical: stat("2026-10-20", {
				earliest: "2026-10-10",
				latest: "2026-11-01",
			}),
		}),
		done_on: null,
		conflict: null,
		guidance: [`Guidance ${id}`],
		...overrides,
	};
}

/** Every day of `year` with flat values; callers overwrite what they need. */
export function seriesFor(year: number): SoilSeriesDay[] {
	const days: SoilSeriesDay[] = [];
	const d = new Date(Date.UTC(year, 0, 1));
	while (d.getUTCFullYear() === year) {
		days.push({
			date: d.toISOString().slice(0, 10),
			this_year: null,
			forecast: null,
			last_year: 60,
			typical: 61,
		});
		d.setUTCDate(d.getUTCDate() + 1);
	}
	return days;
}

export function response(
	overrides: Partial<TimingResponse> = {},
): TimingResponse {
	return {
		generated_at: "2026-09-20T12:00:00Z",
		today: "2026-09-20",
		station: "NOAA USCRN station 3761",
		soil: { avg_5day_f: 66.24, as_of: "2026-09-19", depth_cm: 5 },
		freeze: {
			last_spring: stat("2026-04-10", { p90: "2026-04-25", sample_count: 12 }),
			first_fall: stat("2026-11-05", { p10: "2026-10-22", sample_count: 12 }),
			last_spring_this_year: null,
			first_fall_this_year: null,
		},
		windows: [
			timingWindow("spring_pre_emergent", "NotYet"),
			timingWindow("fall_pre_emergent", "Ideal"),
			timingWindow("fall_seeding", "Closing"),
			timingWindow("spring_seeding", "Closed"),
			timingWindow("dormant_seeding", "NotYet"),
		],
		context: {
			air_anomaly_30d_f: null,
			soil_anomaly_7d_f: 1.5,
			summary:
				"5 cm soil over the last week is running 1.5°F warmer than normal.",
			forecast_alerts: ["Freeze in the forecast: low of 30°F on Sep 24."],
		},
		series: seriesFor(2026),
		history_years: [2020, 2021, 2022, 2023, 2024, 2025],
		data_notes: ["Forecast unavailable — no soil outlook or weather alerts."],
		...overrides,
	};
}
