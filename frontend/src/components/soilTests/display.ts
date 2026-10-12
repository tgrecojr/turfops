// What the Soil Tests page says beside the numbers: level words, the age of the test the
// advice reads, the retest suggestion, the nothing-to-apply case and delete confirmation.

import type { NpkRecommendation, NutrientLevel, SoilTest } from "../../types";
import { formatFullDate } from "../../utils/dates";

/** Title of the table that lists low calcium, magnesium, sulfur and micronutrients. */
// @spec SOIL-UI-018
export const DEFICIENCY_TITLE = "Secondary & micronutrients";

/** Extension guidance for turf: test every 2–3 years. */
const RETEST_AFTER_YEARS = 3;

const LEVEL_WORDS: Record<NutrientLevel, string> = {
	Low: "Low",
	Adequate: "Adequate",
	High: "High",
	NotTested: "Not tested",
};

/** The word shown with a level's color. */
// @spec SOIL-UI-009
export function levelWord(level: NutrientLevel): string {
	return LEVEL_WORDS[level];
}

/** Whole months from `from` to `to` (both `YYYY-MM-DD`). */
function monthsBetween(from: string, to: string): number {
	const [fy, fm, fd] = from.split("-").map(Number);
	const [ty, tm, td] = to.split("-").map(Number);
	return (ty - fy) * 12 + (tm - fm) - (td < fd ? 1 : 0);
}

/** How old a test dated `testDate` is on `today` (both `YYYY-MM-DD`): "7 months". */
// @spec SOIL-UI-014
export function testAge(testDate: string, today: string): string {
	const months = monthsBetween(testDate, today);
	if (months < 1) return "less than a month";
	const years = Math.floor(months / 12);
	const rest = months % 12;
	const unit = (n: number, word: string) => `${n} ${word}${n === 1 ? "" : "s"}`;
	if (years === 0) return unit(rest, "month");
	return rest === 0
		? unit(years, "year")
		: `${unit(years, "year")} ${unit(rest, "month")}`;
}

/** Whether a test dated `testDate` is more than 3 years old on `today`. */
// @spec SOIL-UI-015
export function needsRetest(testDate: string, today: string): boolean {
	const [y, m, d] = testDate.split("-");
	const anniversary = `${Number(y) + RETEST_AFTER_YEARS}-${m}-${d}`;
	return today > anniversary;
}

/** "Based on your test of Mar 23, 2026 (6 months old)". */
// @spec SOIL-UI-012
export function adviceBasis(testDate: string, today: string): string {
	return `Based on your test of ${formatFullDate(testDate)} (${testAge(testDate, today)} old)`;
}

/** Whether the N-P-K advice asks for no N, P₂O₅ or K₂O. */
// @spec SOIL-UI-016
export function nothingToApply(npk: NpkRecommendation): boolean {
	return (
		npk.nitrogen_rate_lbs_per_1000sqft === 0 &&
		npk.phosphorus_rate_lbs_per_1000sqft === 0 &&
		npk.potassium_rate_lbs_per_1000sqft === 0
	);
}

/** The question asked before a history row is deleted. */
// @spec SOIL-UI-017
export function deleteConfirmText(test: SoilTest): string {
	return `Delete the soil test of ${formatFullDate(test.test_date)}? This can't be undone, and it may be the test your advice is based on.`;
}
