import { describe, expect, it } from "vitest";
import type { ActivityStatus, PlannedActivity } from "../../types";
import {
	activityLabel,
	countdown,
	daysBetween,
	STATUS_META,
	STATUS_ORDER,
	shiftMonth,
	statusText,
	yearChoices,
} from "./activityStatus";

function activity(
	status: ActivityStatus,
	start: string,
	end: string,
	estimated = false,
): PlannedActivity {
	return {
		id: "grub_preventative",
		name: "Grub Preventative",
		category: "Pest Control",
		description: "",
		date_window: {
			predicted_start: start,
			predicted_end: end,
			earliest_historical: null,
			latest_historical: null,
			confidence: "High",
		},
		status,
		status_estimated: estimated,
		details: {
			soil_temp_trigger: null,
			product_suggestions: [],
			rate: null,
			notes: null,
		},
	};
}

describe("status display", () => {
	// @spec PLAN-UI-004, PLAN-CALUI-006
	it("orders five statuses, each with a symbol and label", () => {
		expect(STATUS_ORDER).toEqual([
			"Active",
			"Upcoming",
			"Completed",
			"Skipped",
			"Missed",
		]);
		expect(STATUS_ORDER.map((s) => STATUS_META[s].symbol)).toEqual([
			"●",
			"○",
			"✓",
			"–",
			"✗",
		]);
		expect(STATUS_META.Skipped.label).toBe("Not needed");
		expect(STATUS_META.Active.label).toBe("Active");
	});

	// @spec PLAN-UI-011
	it("marks an estimated status", () => {
		expect(statusText(activity("Upcoming", "2026-06-01", "2026-07-01"))).toBe(
			"○ Upcoming",
		);
		expect(
			statusText(activity("Upcoming", "2026-06-01", "2026-07-01", true)),
		).toBe("○ Upcoming · estimate");
	});

	// @spec PLAN-CALUI-005, PLAN-CALUI-009
	it("labels a calendar bar with name, symbol and status, never color alone", () => {
		expect(activityLabel(activity("Active", "2026-06-01", "2026-07-01"))).toBe(
			"Grub Preventative — ● Active",
		);
		expect(
			activityLabel(activity("Skipped", "2026-09-01", "2026-09-21", true)),
		).toBe("Grub Preventative — – Not needed (estimate)");
	});
});

describe("countdown from the plan's today", () => {
	it("counts whole calendar days", () => {
		expect(daysBetween("2026-10-11", "2026-10-12")).toBe(1);
		expect(daysBetween("2026-03-07", "2026-03-09")).toBe(2); // across DST
		expect(daysBetween("2026-10-11", "2026-10-11")).toBe(0);
	});

	// @spec PLAN-UI-007
	it("reads per status", () => {
		const today = "2026-06-10";
		expect(
			countdown(activity("Upcoming", "2026-06-11", "2026-07-01"), today),
		).toBe("Starts tomorrow");
		expect(
			countdown(activity("Upcoming", "2026-06-15", "2026-07-01"), today),
		).toBe("Starts in 5 days");
		expect(
			countdown(activity("Active", "2026-06-01", "2026-06-13"), today),
		).toBe("3 days remaining");
		expect(
			countdown(activity("Active", "2026-06-01", "2026-06-10"), today),
		).toBe("Window closing today");
		expect(
			countdown(activity("Missed", "2026-05-01", "2026-06-01"), today),
		).toBe("Window has passed");
		expect(
			countdown(activity("Completed", "2026-05-01", "2026-06-01"), today),
		).toBe("Done");
		expect(
			countdown(activity("Skipped", "2026-09-01", "2026-09-21"), today),
		).toBe("Not needed this season");
	});
});

describe("server-dated navigation", () => {
	// @spec PLAN-UI-001
	it("offers the year before, the server's year and the year after", () => {
		expect(yearChoices(2026)).toEqual([2025, 2026, 2027]);
	});

	// @spec PLAN-CALUI-001
	it("moves months across year boundaries", () => {
		expect(shiftMonth({ year: 2026, month: 1 }, -1)).toEqual({
			year: 2025,
			month: 12,
		});
		expect(shiftMonth({ year: 2026, month: 12 }, 1)).toEqual({
			year: 2027,
			month: 1,
		});
		expect(shiftMonth({ year: 2026, month: 6 }, 1)).toEqual({
			year: 2026,
			month: 7,
		});
	});
});
