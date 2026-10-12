import { describe, expect, it } from "vitest";
import type { ApplicationType } from "./index";
import { APPLICATION_TYPE_LABELS } from "./index";
import type { ProductCategory } from "./inventory";
import { PRODUCT_CATEGORIES } from "./inventory";
import { categoriesFor, guessCategory } from "./inventoryCompat";

// The backend's `ProductCategory::accepts` table (INV-SHELF-005), one row per category.
const ACCEPTS: Record<ProductCategory, ApplicationType[]> = {
	Fertilizer: ["Fertilizer", "PlantFertilizer"],
	Supplement: ["Fertilizer", "PlantFertilizer"],
	Fungicide: ["Fungicide"],
	Herbicide: ["PreEmergent", "PostEmergent"],
	InsectControl: ["GrubControl", "Insecticide"],
	Seed: ["Overseed"],
	SoilAmendment: ["Lime", "Sulfur"],
	Surfactant: ["Wetting"],
	Other: Object.keys(APPLICATION_TYPE_LABELS) as ApplicationType[],
};

function accepts(c: ProductCategory, t: ApplicationType): boolean {
	return t === "Other" || ACCEPTS[c].includes(t);
}

// @spec INV-SHELF-006
describe("categoriesFor", () => {
	const types = Object.keys(APPLICATION_TYPE_LABELS) as ApplicationType[];

	it.each(types)("offers exactly what accepts allows under %s", (t) => {
		const expected = PRODUCT_CATEGORIES.filter((c) => accepts(c, t));
		expect([...categoriesFor(t)].sort()).toEqual([...expected].sort());
		expect(categoriesFor(t)).toContain("Other");
	});
});

// @spec INV-LOG-003
describe("guessCategory", () => {
	it("takes the first specific category the logged types allow", () => {
		expect(guessCategory(["Mowing", "GrubControl", "Fungicide"])).toBe(
			"InsectControl",
		);
		expect(guessCategory(["PlantFertilizer"])).toBe("Fertilizer");
	});

	it("leaves the category open when no type names one", () => {
		expect(guessCategory(["Mowing", "Aeration"])).toBeUndefined();
	});
});
