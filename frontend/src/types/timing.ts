// Mirrors backend/src/models/timing.rs

import type { WindowConfidence } from ".";

export type WindowId =
	| "fall_seeding"
	| "spring_pre_emergent"
	| "fall_pre_emergent"
	| "spring_seeding"
	| "dormant_seeding";

export type WindowPriority = "Primary" | "Secondary" | "Optional";

export type WindowState =
	| "NotYet"
	| "OpeningSoon"
	| "Open"
	| "Ideal"
	| "Closing"
	| "Closed"
	| "Done"
	| "Blocked";

export type DateSource = "Observed" | "Tentative" | "Forecast" | "Typical";

export interface DateStat {
	median: string;
	p10: string;
	p90: string;
	earliest: string;
	latest: string;
	sample_count: number;
	confidence: WindowConfidence;
}

export interface BoundaryView {
	label: string;
	typical: DateStat | null;
	/** Null when the trigger is overdue versus its typical date. */
	date: string | null;
	source: DateSource;
	passed: boolean;
	days_held: number | null;
}

export interface TimingWindow {
	id: WindowId;
	name: string;
	priority: WindowPriority;
	season_year: number;
	state: WindowState;
	headline: string;
	detail: string;
	opens: BoundaryView;
	ideal_from: BoundaryView | null;
	ideal_until: BoundaryView | null;
	closes: BoundaryView;
	done_on: string | null;
	conflict: string | null;
	guidance: string[];
}

export interface SoilNow {
	avg_5day_f: number | null;
	as_of: string | null;
	depth_cm: number;
}

export interface FreezeDates {
	last_spring: DateStat | null;
	first_fall: DateStat | null;
	last_spring_this_year: string | null;
	first_fall_this_year: string | null;
}

export interface SeasonContext {
	air_anomaly_30d_f: number | null;
	soil_anomaly_7d_f: number | null;
	summary: string | null;
	forecast_alerts: string[];
}

export interface SoilSeriesDay {
	date: string;
	this_year: number | null;
	forecast: number | null;
	last_year: number | null;
	typical: number | null;
}

export interface TimingResponse {
	generated_at: string;
	today: string;
	station: string;
	soil: SoilNow;
	freeze: FreezeDates;
	windows: TimingWindow[];
	context: SeasonContext;
	series: SoilSeriesDay[];
	history_years: number[];
	data_notes: string[];
}

export const WINDOW_STATE_LABELS: Record<WindowState, string> = {
	NotYet: "Not yet",
	OpeningSoon: "Opening soon",
	Open: "Open",
	Ideal: "Ideal now",
	Closing: "Closing",
	Closed: "Closed",
	Done: "Done",
	Blocked: "Skip",
};

// Status palette shared with disease risk (good / warning) plus neutrals. State is
// never carried by color alone — always pair with the symbol + label.
export const WINDOW_STATE_COLORS: Record<WindowState, string> = {
	NotYet: "#a0aec0",
	OpeningSoon: "#3182ce",
	Open: "#0ca30c",
	Ideal: "#0ca30c",
	Closing: "#fab219",
	Closed: "#718096",
	Done: "#3182ce",
	Blocked: "#718096",
};

export const WINDOW_STATE_SYMBOLS: Record<WindowState, string> = {
	NotYet: "○",
	OpeningSoon: "◔",
	Open: "●",
	Ideal: "★",
	Closing: "△",
	Closed: "✕",
	Done: "✓",
	Blocked: "⊘",
};

export const DATE_SOURCE_LABELS: Record<DateSource, string> = {
	Observed: "observed",
	Tentative: "tentative",
	Forecast: "forecast",
	Typical: "typical",
};

/** Windows that need attention now, most actionable first. */
const STATE_RANK: Record<WindowState, number> = {
	Closing: 0,
	Ideal: 1,
	Open: 2,
	OpeningSoon: 3,
	NotYet: 4,
	Done: 5,
	Blocked: 6,
	Closed: 7,
};

export function byUrgency(a: TimingWindow, b: TimingWindow): number {
	return STATE_RANK[a.state] - STATE_RANK[b.state];
}

const MONTHS = [
	"Jan",
	"Feb",
	"Mar",
	"Apr",
	"May",
	"Jun",
	"Jul",
	"Aug",
	"Sep",
	"Oct",
	"Nov",
	"Dec",
];

/** Format an ISO `YYYY-MM-DD` date as `Sep 18` without timezone shifts. */
export function formatMonthDay(iso: string): string {
	const [, month, day] = iso.split("-");
	return `${MONTHS[Number(month) - 1]} ${Number(day)}`;
}

/** Days since the epoch for an ISO date, for positioning on a time axis. */
export function dayNumber(iso: string): number {
	const [year, month, day] = iso.split("-").map(Number);
	return Math.round(Date.UTC(year, month - 1, day) / 86_400_000);
}
