// Target vocabulary and refresh-diff labels for the product inventory.
import type { ProductCategory, ProductTarget } from "./inventory";

export const PRODUCT_TARGET_LABELS: Record<ProductTarget, string> = {
	Grubs: "Grubs",
	SurfaceInsects: "Surface insects",
	Ants: "Ants",
	Termites: "Termites",
	Ticks: "Ticks",
	Mosquitoes: "Mosquitoes",
	Broadleaf: "Broadleaf weeds",
	Crabgrass: "Crabgrass",
	Nutsedge: "Nutsedge",
	Poa: "Poa annua",
	Moss: "Moss",
	Iron: "Iron",
	Manganese: "Manganese",
	Magnesium: "Magnesium",
	Zinc: "Zinc",
	Boron: "Boron",
	Copper: "Copper",
	Calcium: "Calcium",
	Kbg: "Kentucky bluegrass",
	Ttf: "Tall fescue",
	Prg: "Perennial ryegrass",
	FineFescue: "Fine fescue",
	Bermuda: "Bermuda",
	Zoysia: "Zoysia",
};

/** Targets that make sense for a category — the edit form shows only these. */
export const TARGETS_BY_CATEGORY: Record<ProductCategory, ProductTarget[]> = {
	InsectControl: [
		"Grubs",
		"SurfaceInsects",
		"Ants",
		"Termites",
		"Ticks",
		"Mosquitoes",
	],
	Herbicide: ["Broadleaf", "Crabgrass", "Nutsedge", "Poa", "Moss"],
	Supplement: [
		"Iron",
		"Manganese",
		"Magnesium",
		"Zinc",
		"Boron",
		"Copper",
		"Calcium",
	],
	Fertilizer: ["Iron", "Manganese", "Magnesium", "Zinc"],
	SoilAmendment: ["Calcium", "Magnesium"],
	Seed: ["Kbg", "Ttf", "Prg", "FineFescue", "Bermuda", "Zoysia"],
	Fungicide: [],
	Surfactant: [],
	Other: Object.keys(PRODUCT_TARGET_LABELS) as ProductTarget[],
};

export const CHANGED_FIELD_LABELS: Record<string, string> = {
	category: "category",
	form: "form",
	label_rate: "suggested rate",
	npk: "N-P-K",
	frac_classes: "FRAC classes",
	herbicide_timing: "herbicide timing",
	targets: "targets",
	amendment_kind: "amendment kind",
};
