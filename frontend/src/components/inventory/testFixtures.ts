// Products and profiles for the inventory component tests.
import type { Product, ProductProfile } from "../../types/inventory";

export function makeProduct(overrides: Partial<Product> = {}): Product {
	return {
		id: 1,
		lawn_profile_id: 1,
		name: "Milorganite",
		brand: null,
		stock_status: "InStock",
		category: "Fertilizer",
		form: "Granular",
		label_rate_per_1000sqft: null,
		label_rate_unit: null,
		nitrogen_pct: null,
		phosphorus_pct: null,
		potassium_pct: null,
		frac_classes: [],
		herbicide_timing: null,
		targets: [],
		amendment_kind: null,
		profile: null,
		profile_generated_at: null,
		profile_model: null,
		notes: null,
		archived: false,
		created_at: "2026-09-01T12:00:00Z",
		updated_at: "2026-09-01T12:00:00Z",
		...overrides,
	};
}

export function makeProfile(
	overrides: Partial<ProductProfile> = {},
): ProductProfile {
	return {
		identified_name: "Heritage G",
		manufacturer: "Syngenta",
		category: "Fungicide",
		form: "Granular",
		active_ingredients: [{ name: "azoxystrobin", pct: 0.31 }],
		npk: null,
		frac_classes: ["Frac11"],
		herbicide_timing: null,
		targets: [],
		amendment_kind: null,
		mobility: "Systemic",
		nitrogen_release: "NotApplicable",
		suitable_for: "Turf",
		suggested_label_rate: { amount: 2, unit: "Lb" },
		summary: "A strobilurin fungicide for brown patch.",
		cautions: [],
		confidence: "High",
		...overrides,
	};
}
