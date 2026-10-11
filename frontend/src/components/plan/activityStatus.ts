// How a planned activity's status reads everywhere it is shown: symbol + label (never
// color alone), the countdown, and the estimate marker.

import type { ActivityStatus, PlannedActivity } from "../../types";

export interface StatusMeta {
	symbol: string;
	label: string;
	color: string;
}

/** Display order for summary chips and legends. */
// @spec PLAN-UI-004, PLAN-CALUI-006
export const STATUS_ORDER: ActivityStatus[] = [
	"Active",
	"Upcoming",
	"Completed",
	"Skipped",
	"Missed",
];

export const STATUS_META: Record<ActivityStatus, StatusMeta> = {
	Active: { symbol: "●", label: "Active", color: "#22c55e" },
	Upcoming: { symbol: "○", label: "Upcoming", color: "#3b82f6" },
	Completed: { symbol: "✓", label: "Completed", color: "#6b7280" },
	Skipped: { symbol: "–", label: "Not needed", color: "#a0aec0" },
	Missed: { symbol: "✗", label: "Missed", color: "#ef4444" },
};

const DAY_MS = 24 * 60 * 60 * 1000;

/** Whole days from `from` to `to` (both YYYY-MM-DD), counted on the calendar. */
export function daysBetween(from: string, to: string): number {
	return Math.round(
		(Date.parse(`${to}T00:00:00Z`) - Date.parse(`${from}T00:00:00Z`)) / DAY_MS,
	);
}

/** The card's countdown, counted from the plan's `today` (never the browser's clock). */
// @spec PLAN-UI-007
export function countdown(activity: PlannedActivity, today: string): string {
	const { predicted_start, predicted_end } = activity.date_window;
	switch (activity.status) {
		case "Upcoming": {
			const days = daysBetween(today, predicted_start);
			return days === 1 ? "Starts tomorrow" : `Starts in ${days} days`;
		}
		case "Active": {
			const days = daysBetween(today, predicted_end);
			return days <= 0 ? "Window closing today" : `${days} days remaining`;
		}
		case "Missed":
			return "Window has passed";
		case "Skipped":
			return "Not needed this season";
		case "Completed":
			return "Done";
	}
}

/** "● Active", with "· estimate" when the status is a typical-date estimate. */
// @spec PLAN-UI-011
export function statusText(activity: PlannedActivity): string {
	const meta = STATUS_META[activity.status];
	const text = `${meta.symbol} ${meta.label}`;
	return activity.status_estimated ? `${text} · estimate` : text;
}

/** Title and accessible name for a calendar bar: name, status symbol and label. */
// @spec PLAN-CALUI-005, PLAN-CALUI-009
export function activityLabel(activity: PlannedActivity): string {
	const meta = STATUS_META[activity.status];
	const estimate = activity.status_estimated ? " (estimate)" : "";
	return `${activity.name} — ${meta.symbol} ${meta.label}${estimate}`;
}

/** The year buttons: the year before, the server's year, the year after. */
// @spec PLAN-UI-001
export function yearChoices(serverYear: number): number[] {
	return [serverYear - 1, serverYear, serverYear + 1];
}

/** Move a calendar month by `delta`, across year boundaries. */
// @spec PLAN-CALUI-001
export function shiftMonth(
	at: { year: number; month: number },
	delta: number,
): { year: number; month: number } {
	const index = at.year * 12 + (at.month - 1) + delta;
	return { year: Math.floor(index / 12), month: (index % 12) + 1 };
}
