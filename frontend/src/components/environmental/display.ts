// What the Environmental page shows beside the numbers: chart reference lines, forecast
// day completeness, the station's observation time, and crossing timing.

import type {
	DailyForecast,
	EnvironmentalSummary,
	ThresholdPrediction,
	TimeSeriesPoint,
} from "../../types";
import { formatMonthDay } from "../../utils/dates";

export interface ReferenceLine {
	value: number;
	label: string;
}

/** Reference lines per trend chart; a chart missing here draws none. Soil has none:
 * pre-emergent and seeding timing are judged on 5 cm soil on the Timing page. */
// @spec WX-VIEW-PAGE-007
export const TREND_REFERENCES: Record<string, ReferenceLine | undefined> = {
	ambient: { value: 85, label: "Heat stress" },
	humidity: { value: 80, label: "Humid (80%)" },
	moisture: { value: 0.1, label: "Drought" },
};

/** Whether a forecast card covers less than the whole day. */
// @spec WX-VIEW-PAGE-009
export function isPartialDay(day: DailyForecast): boolean {
	return day.hours_covered !== undefined && day.hours_covered < 24;
}

/** "Station soil observed Oct 10, 2:00 PM", or null with no observation. */
// @spec WX-VIEW-PAGE-008
export function soilObservedLine(summary: EnvironmentalSummary): string | null {
	if (!summary.soil_observed_at) return null;
	const at = new Date(summary.soil_observed_at);
	if (Number.isNaN(at.getTime())) return null;
	const when = at.toLocaleString(undefined, {
		month: "short",
		day: "numeric",
		hour: "numeric",
		minute: "2-digit",
	});
	return `Station soil observed ${when}`;
}

/** The banner shown while the station's soil data is too old to describe today. */
export function staleSoilMessage(summary: EnvironmentalSummary): string {
	const at = summary.soil_observed_at
		? new Date(summary.soil_observed_at).toLocaleDateString()
		: "an earlier date";
	return `Station soil data stopped on ${at}. Soil readings are hidden and soil-based advice is paused until the feed catches up.`;
}

/** The recent line the prediction chart plots: the last `days` days of the series. */
// @spec WX-OUTLOOK-UI-004
export function lastDays(
	points: TimeSeriesPoint[],
	days: number,
): TimeSeriesPoint[] {
	if (points.length === 0) return points;
	const newest = new Date(points[points.length - 1].timestamp).getTime();
	const cutoff = newest - days * 86_400_000;
	return points.filter((p) => new Date(p.timestamp).getTime() >= cutoff);
}

/** "~3 days (Oct 14)", or "beyond the forecast" when the estimate has no date. */
// @spec WX-OUTLOOK-CROSS-005
export function crossingWhen(c: ThresholdPrediction): string {
	if (c.beyond_forecast || !c.estimated_crossing_date) {
		return "beyond the forecast";
	}
	const days = c.days_until_crossing;
	return `~${days} ${days === 1 ? "day" : "days"} (${formatMonthDay(c.estimated_crossing_date)})`;
}
