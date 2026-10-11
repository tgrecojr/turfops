import { describe, expect, it } from "vitest";
import { PRODUCT_TARGET_LABELS, TARGETS_BY_CATEGORY } from "./inventoryTargets";

const NUTRIENTS = [
	"Iron",
	"Manganese",
	"Magnesium",
	"Zinc",
	"Boron",
	"Copper",
	"Calcium",
];

describe("edit-form targets", () => {
	// @spec INV-UI-007
	it("offers exactly the targets the backend keeps for each category", () => {
		expect(TARGETS_BY_CATEGORY.InsectControl).toEqual([
			"Grubs",
			"SurfaceInsects",
			"Ants",
			"Termites",
			"Ticks",
			"Mosquitoes",
		]);
		expect(TARGETS_BY_CATEGORY.Herbicide).toEqual([
			"Broadleaf",
			"Crabgrass",
			"Nutsedge",
			"Poa",
			"Moss",
		]);
		expect(TARGETS_BY_CATEGORY.Fertilizer).toEqual(NUTRIENTS);
		expect(TARGETS_BY_CATEGORY.Supplement).toEqual(NUTRIENTS);
		expect(TARGETS_BY_CATEGORY.Seed).toEqual([
			"Kbg",
			"Ttf",
			"Prg",
			"FineFescue",
			"Bermuda",
			"Zoysia",
		]);
		expect(TARGETS_BY_CATEGORY.Fungicide).toEqual([]);
		expect(TARGETS_BY_CATEGORY.SoilAmendment).toEqual([]);
		expect(TARGETS_BY_CATEGORY.Surfactant).toEqual([]);
		expect(TARGETS_BY_CATEGORY.Other).toEqual(
			Object.keys(PRODUCT_TARGET_LABELS),
		);
	});
});
