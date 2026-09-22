// Which shelf categories an application type may draw from.
import type { ApplicationType } from "./index";
import type { ProductCategory } from "./inventory";
import { PRODUCT_CATEGORIES } from "./inventory";

/**
 * Categories that can be logged under an application type — mirrors
 * `ProductCategory::accepts` in the backend. `Other` on either side accepts anything.
 */
export function categoriesFor(t: ApplicationType): ProductCategory[] {
	if (t === "Other") return PRODUCT_CATEGORIES;
	const specific: ProductCategory[] = (() => {
		switch (t) {
			case "Fertilizer":
			case "PlantFertilizer":
				return ["Fertilizer", "Supplement"];
			case "Fungicide":
				return ["Fungicide"];
			case "PreEmergent":
			case "PostEmergent":
				return ["Herbicide"];
			case "GrubControl":
			case "Insecticide":
				return ["InsectControl"];
			case "Overseed":
				return ["Seed"];
			case "Lime":
			case "Sulfur":
				return ["SoilAmendment"];
			case "Wetting":
				return ["Surfactant"];
			default:
				return [];
		}
	})();
	return [...specific, "Other"];
}

/** The category a logged name most likely belongs to, from how it was logged. */
export function guessCategory(
	types: ApplicationType[],
): ProductCategory | undefined {
	for (const t of types) {
		const c = categoriesFor(t)[0];
		if (c !== "Other") return c;
	}
	return undefined;
}
