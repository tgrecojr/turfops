import { describe, expect, it } from "vitest";
import type { NpkRecommendation, SoilTest } from "../../types";
import {
	adviceBasis,
	DEFICIENCY_TITLE,
	deleteConfirmText,
	levelWord,
	needsRetest,
	nothingToApply,
	testAge,
} from "./display";

describe("nutrient levels", () => {
	// @spec SOIL-UI-009
	it("are named in words, Not tested included", () => {
		expect(levelWord("Low")).toBe("Low");
		expect(levelWord("Adequate")).toBe("Adequate");
		expect(levelWord("High")).toBe("High");
		expect(levelWord("NotTested")).toBe("Not tested");
	});
});

describe("the test the advice reads", () => {
	// @spec SOIL-UI-012, SOIL-UI-014
	it("is named with its date and age", () => {
		expect(adviceBasis("2026-03-23", "2026-10-11")).toBe(
			"Based on your test of Mar 23, 2026 (6 months old)",
		);
	});

	// @spec SOIL-UI-014
	it("ages in whole months and years", () => {
		expect(testAge("2026-10-01", "2026-10-11")).toBe("less than a month");
		expect(testAge("2026-09-11", "2026-10-11")).toBe("1 month");
		expect(testAge("2026-09-12", "2026-10-11")).toBe("less than a month");
		expect(testAge("2025-10-11", "2026-10-11")).toBe("1 year");
		expect(testAge("2024-03-23", "2026-10-11")).toBe("2 years 6 months");
	});

	// @spec SOIL-UI-015
	it("suggests a retest only past 3 years", () => {
		expect(needsRetest("2023-10-11", "2026-10-11")).toBe(false);
		expect(needsRetest("2023-10-10", "2026-10-11")).toBe(true);
		expect(needsRetest("2026-03-23", "2026-10-11")).toBe(false);
	});
});

describe("fertilizer advice", () => {
	const npk = (n: number, p: number, k: number): NpkRecommendation =>
		({
			nitrogen_rate_lbs_per_1000sqft: n,
			phosphorus_rate_lbs_per_1000sqft: p,
			potassium_rate_lbs_per_1000sqft: k,
		}) as NpkRecommendation;

	// @spec SOIL-UI-016
	it("is 'nothing to apply' only when N, P₂O₅ and K₂O are all zero", () => {
		expect(nothingToApply(npk(0, 0, 0))).toBe(true);
		expect(nothingToApply(npk(0, 0.25, 0))).toBe(false);
		expect(nothingToApply(npk(0.75, 0, 0))).toBe(false);
	});
});

describe("history", () => {
	// @spec SOIL-UI-017
	it("asks before deleting, naming the test's date", () => {
		const text = deleteConfirmText({ test_date: "2026-03-23" } as SoilTest);
		expect(text).toContain("Mar 23, 2026");
		expect(text).toMatch(/can't be undone/);
	});

	// @spec SOIL-UI-018
	it("titles the deficiency table for secondary nutrients too", () => {
		expect(DEFICIENCY_TITLE).toBe("Secondary & micronutrients");
	});
});
