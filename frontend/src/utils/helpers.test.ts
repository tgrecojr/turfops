import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { categoryLabel } from "../types";
import {
	addDaysISO,
	formatDateRange,
	formatFullDate,
	formatMonthDay,
	formatShortDate,
	todayLocalISO,
} from "./dates";
import { formatInches, mmToInches } from "./units";

// West of UTC, so a date read as UTC midnight shows the day before and the UTC date runs
// a day ahead in the evening.
const env = (
	globalThis as unknown as {
		process: { env: Record<string, string | undefined> };
	}
).process.env;
let savedTz: string | undefined;
beforeEach(() => {
	savedTz = env.TZ;
	env.TZ = "America/Los_Angeles";
});
afterEach(() => {
	if (savedTz === undefined) delete env.TZ;
	else env.TZ = savedTz;
	vi.useRealTimers();
});

describe("today for a form date", () => {
	// @spec UI-UTIL-001
	it("is the browser's local calendar date, not the UTC date", () => {
		vi.useFakeTimers();
		// 7:30 pm on Oct 11 in Los Angeles is already Oct 12 in UTC.
		vi.setSystemTime(new Date("2026-10-12T02:30:00Z"));
		expect(new Date().toISOString().slice(0, 10)).toBe("2026-10-12");
		expect(todayLocalISO()).toBe("2026-10-11");
		expect(addDaysISO("2026-10-31", 1)).toBe("2026-11-01");
	});
});

describe("an API calendar date on screen", () => {
	// @spec UI-UTIL-002
	it("shows the same calendar day in any timezone", () => {
		expect(new Date("2026-10-01").getDate()).toBe(30); // the bug these avoid
		expect(formatShortDate("2026-10-01")).toBe("10/1");
		expect(formatMonthDay("2026-10-01")).toBe("Oct 1");
		expect(formatFullDate("2026-01-01")).toBe("Jan 1, 2026");
		expect(formatDateRange("2026-09-01", "2026-10-01")).toBe("Sep 1 – Oct 1");
	});
});

describe("precipitation", () => {
	// @spec UI-UTIL-003
	it("converts millimetres to inches, two decimals with 'in' by default", () => {
		expect(mmToInches(25.4)).toBe(1);
		expect(formatInches(25.4)).toBe("1.00 in");
		expect(formatInches(3)).toBe("0.12 in");
		expect(formatInches(50.8, 1)).toBe("2.0 in");
	});
});

describe("recommendation category", () => {
	// @spec UI-UTIL-004
	it("splits the PascalCase variant into words", () => {
		expect(categoryLabel("DiseasePressure")).toBe("Disease Pressure");
		expect(categoryLabel("SoilTempForecast")).toBe("Soil Temp Forecast");
		expect(categoryLabel("Mowing")).toBe("Mowing");
	});
});
