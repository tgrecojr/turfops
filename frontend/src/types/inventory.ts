// Product inventory — mirrors backend/src/models/product/*.rs
import type { ApplicationType } from "./index";

export type ProductCategory =
	| "Fertilizer"
	| "Supplement"
	| "Fungicide"
	| "Herbicide"
	| "InsectControl"
	| "Seed"
	| "SoilAmendment"
	| "Surfactant"
	| "Other";

export type ProductForm =
	| "Granular"
	| "Liquid"
	| "WaterSoluble"
	| "Seed"
	| "Other";

export type StockStatus = "InStock" | "Low" | "Out";

export type HerbicideTiming = "PreEmergent" | "PostEmergent" | "Both";

export type AmendmentKind =
	| "CalciticLime"
	| "DolomiticLime"
	| "Sulfur"
	| "Gypsum"
	| "Humic"
	| "Compost"
	| "Other";

export type RateUnit = "Lb" | "Oz" | "FlOz";

export type Mobility = "Systemic" | "Contact" | "NotApplicable";
export type NitrogenRelease = "Quick" | "Slow" | "Mixed" | "NotApplicable";
export type SuitableFor = "Turf" | "Ornamentals" | "Both";

export type ProductTarget =
	| "Grubs"
	| "SurfaceInsects"
	| "Ants"
	| "Termites"
	| "Ticks"
	| "Mosquitoes"
	| "Broadleaf"
	| "Crabgrass"
	| "Nutsedge"
	| "Poa"
	| "Moss"
	| "Iron"
	| "Manganese"
	| "Magnesium"
	| "Zinc"
	| "Boron"
	| "Copper"
	| "Calcium"
	| "Kbg"
	| "Ttf"
	| "Prg"
	| "FineFescue"
	| "Bermuda"
	| "Zoysia";

export type ProfileConfidence = "High" | "Medium" | "Low";

export interface ActiveIngredient {
	name: string;
	pct: number | null;
}

export interface ProductProfile {
	identified_name: string;
	manufacturer: string | null;
	category: ProductCategory;
	form: ProductForm;
	active_ingredients: ActiveIngredient[];
	npk: { n: number; p: number; k: number } | null;
	frac_classes: string[];
	herbicide_timing: HerbicideTiming | null;
	targets: ProductTarget[];
	amendment_kind: AmendmentKind | null;
	mobility: Mobility;
	nitrogen_release: NitrogenRelease;
	suitable_for: SuitableFor;
	suggested_label_rate: { amount: number; unit: RateUnit } | null;
	summary: string;
	cautions: string[];
	confidence: ProfileConfidence;
}

/** User-owned, matchable facts (flattened into Product on the wire). */
export interface ProductFacts {
	category: ProductCategory;
	form: ProductForm;
	label_rate_per_1000sqft: number | null;
	label_rate_unit: RateUnit | null;
	nitrogen_pct: number | null;
	phosphorus_pct: number | null;
	potassium_pct: number | null;
	frac_classes: string[];
	herbicide_timing: HerbicideTiming | null;
	targets: ProductTarget[];
	amendment_kind: AmendmentKind | null;
}

export interface Product extends ProductFacts {
	id: number | null;
	lawn_profile_id: number;
	name: string;
	brand: string | null;
	stock_status: StockStatus;
	profile: ProductProfile | null;
	profile_generated_at: string | null;
	profile_model: string | null;
	notes: string | null;
	archived: boolean;
	created_at: string;
	updated_at: string;
}

export interface ProductCreated {
	product: Product;
	/** The assistant was asked for but failed; the product was saved by hand. */
	profile_error: string | null;
}

/** A product name from the application log that is not on the shelf. */
export interface LoggedProductSuggestion {
	name: string;
	uses: number;
	last_used: string;
	application_types: ApplicationType[];
}

export interface ProductRefreshed {
	product: Product;
	suggested_facts: ProductFacts;
	changed_fields: string[];
}

export const PRODUCT_CATEGORIES: ProductCategory[] = [
	"Fertilizer",
	"Supplement",
	"Fungicide",
	"Herbicide",
	"InsectControl",
	"Seed",
	"SoilAmendment",
	"Surfactant",
	"Other",
];

export const PRODUCT_CATEGORY_LABELS: Record<ProductCategory, string> = {
	Fertilizer: "Fertilizer",
	Supplement: "Nutrient supplement",
	Fungicide: "Fungicide",
	Herbicide: "Herbicide",
	InsectControl: "Insect control",
	Seed: "Seed",
	SoilAmendment: "Soil amendment",
	Surfactant: "Surfactant",
	Other: "Other",
};

export const PRODUCT_FORMS: ProductForm[] = [
	"Granular",
	"Liquid",
	"WaterSoluble",
	"Seed",
	"Other",
];

export const PRODUCT_FORM_LABELS: Record<ProductForm, string> = {
	Granular: "Granular",
	Liquid: "Liquid",
	WaterSoluble: "Water-soluble",
	Seed: "Seed",
	Other: "Other",
};

export const STOCK_STATUSES: StockStatus[] = ["InStock", "Low", "Out"];

export const STOCK_STATUS_LABELS: Record<StockStatus, string> = {
	InStock: "In stock",
	Low: "Running low",
	Out: "Out",
};

// Status palette shared with timing / disease risk; never color alone.
export const STOCK_STATUS_COLORS: Record<StockStatus, string> = {
	InStock: "#0ca30c",
	Low: "#fab219",
	Out: "#d03b3b",
};

/** In stock → Low → Out → In stock (the pill's click cycle). */
export function nextStockStatus(status: StockStatus): StockStatus {
	if (status === "InStock") return "Low";
	if (status === "Low") return "Out";
	return "InStock";
}

export const STOCK_STATUS_SYMBOLS: Record<StockStatus, string> = {
	InStock: "✓",
	Low: "△",
	Out: "✕",
};

export const HERBICIDE_TIMINGS: HerbicideTiming[] = [
	"PreEmergent",
	"PostEmergent",
	"Both",
];

export const HERBICIDE_TIMING_LABELS: Record<HerbicideTiming, string> = {
	PreEmergent: "Pre-emergent",
	PostEmergent: "Post-emergent",
	Both: "Pre- and post-emergent",
};

export const AMENDMENT_KINDS: AmendmentKind[] = [
	"CalciticLime",
	"DolomiticLime",
	"Sulfur",
	"Gypsum",
	"Humic",
	"Compost",
	"Other",
];

export const AMENDMENT_KIND_LABELS: Record<AmendmentKind, string> = {
	CalciticLime: "Calcitic lime",
	DolomiticLime: "Dolomitic lime",
	Sulfur: "Sulfur",
	Gypsum: "Gypsum",
	Humic: "Humic",
	Compost: "Compost",
	Other: "Other",
};

export const RATE_UNITS: RateUnit[] = ["Lb", "Oz", "FlOz"];

export const RATE_UNIT_LABELS: Record<RateUnit, string> = {
	Lb: "lb",
	Oz: "oz",
	FlOz: "fl oz",
};

/** Pull the user-owned facts out of a product (the wire format is flat). */
export function factsOf(p: Product): ProductFacts {
	return {
		category: p.category,
		form: p.form,
		label_rate_per_1000sqft: p.label_rate_per_1000sqft,
		label_rate_unit: p.label_rate_unit,
		nitrogen_pct: p.nitrogen_pct,
		phosphorus_pct: p.phosphorus_pct,
		potassium_pct: p.potassium_pct,
		frac_classes: p.frac_classes,
		herbicide_timing: p.herbicide_timing,
		targets: p.targets,
		amendment_kind: p.amendment_kind,
	};
}

// ---- What a recommendation needs from the shelf (mirrors models/inventory.rs) ----

export interface ProductNeed {
	label: string;
	categories: ProductCategory[];
	frac_classes: string[];
	herbicide_timing: HerbicideTiming | null;
	targets: ProductTarget[];
	amendment_kinds: AmendmentKind[];
	optional: boolean;
}

export type InventoryState = "OnHand" | "Low" | "NotOnHand" | "Partial";

export interface ShelfProduct {
	product_id: number;
	name: string;
	stock_status: StockStatus;
}

export interface InventoryMatch extends ShelfProduct {
	/** The need this product satisfies. */
	need: string;
}

export interface InventoryStatus {
	state: InventoryState;
	matches: InventoryMatch[];
	/** Required needs nothing on the shelf covers. */
	missing: string[];
}

export const INVENTORY_STATE_LABELS: Record<InventoryState, string> = {
	OnHand: "On hand",
	Low: "On hand, running low",
	NotOnHand: "Need to buy",
	Partial: "Partly on hand",
};

// Status palette; never color alone — always symbol + label.
export const INVENTORY_STATE_COLORS: Record<InventoryState, string> = {
	OnHand: "#0ca30c",
	Low: "#fab219",
	NotOnHand: "#d03b3b",
	Partial: "#fab219",
};

export const INVENTORY_STATE_SYMBOLS: Record<InventoryState, string> = {
	OnHand: "✓",
	Low: "△",
	NotOnHand: "✕",
	Partial: "◐",
};
