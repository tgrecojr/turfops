// The Landscape page's edit form and expanded-row details, kept free of React.

import type { Plant, PlantType } from "../../types";

export const PLANT_TYPES: PlantType[] = [
	"Shrub",
	"Tree",
	"Perennial",
	"Annual",
	"Vine",
	"Groundcover",
	"Other",
];

export interface PlantFields {
	commonName: string;
	scientificName: string;
	plantType: PlantType;
	location: string;
	plantingDate: string;
	notes: string;
}

export interface PlantUpdate {
	common_name: string;
	scientific_name: string | null;
	plant_type: PlantType;
	location: string | null;
	planting_date: string | null;
	notes: string | null;
}

/** The form's starting values for a plant. */
export function fieldsOf(plant: Plant): PlantFields {
	return {
		commonName: plant.common_name,
		scientificName: plant.scientific_name ?? "",
		plantType: plant.plant_type,
		location: plant.location ?? "",
		plantingDate: plant.planting_date ?? "",
		notes: plant.notes ?? "",
	};
}

/** The `PUT /plants/:id` body, or the message that blocks the save. */
export function editPayload(
	f: PlantFields,
): { payload: PlantUpdate } | { error: string } {
	// @spec PLANT-UI-012
	const commonName = f.commonName.trim();
	if (!commonName) return { error: "Enter the plant's common name." };
	const optional = (v: string) => v.trim() || null;
	return {
		payload: {
			common_name: commonName,
			scientific_name: optional(f.scientificName),
			plant_type: f.plantType,
			location: optional(f.location),
			planting_date: optional(f.plantingDate),
			notes: optional(f.notes),
		},
	};
}

/** Lines shown under an expanded plant's summary. */
export function plantDetails(plant: Plant): string[] {
	// @spec PLANT-UI-013
	const lines: string[] = [];
	if (plant.planting_date) lines.push(`Planted ${plant.planting_date}`);
	if (plant.notes?.trim()) lines.push(`Notes: ${plant.notes.trim()}`);
	return lines;
}
