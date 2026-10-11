import { describe, expect, it } from "vitest";
import type { Plant } from "../../types";
import { editPayload, fieldsOf, plantDetails } from "./plantEdit";

const holly = {
	id: 4,
	lawn_profile_id: 1,
	common_name: "Holly",
	scientific_name: "Ilex opaca",
	plant_type: "Shrub",
	location: "Front walk",
	planting_date: "2024-04-01",
	notes: "Berries in winter",
	plan_generated_at: "2026-01-01T00:00:00Z",
	plan_model: "m",
	created_at: "2026-01-01T00:00:00Z",
	updated_at: "2026-01-01T00:00:00Z",
} as unknown as Plant;

describe("plant edit", () => {
	// @spec PLANT-UI-012
	it("sends every editable field, empty optional ones as null", () => {
		const fields = {
			...fieldsOf(holly),
			commonName: "  American Holly ",
			location: "",
			notes: "",
		};
		expect(editPayload(fields)).toEqual({
			payload: {
				common_name: "American Holly",
				scientific_name: "Ilex opaca",
				plant_type: "Shrub",
				location: null,
				planting_date: "2024-04-01",
				notes: null,
			},
		});
	});

	// @spec PLANT-UI-012
	it("needs a common name", () => {
		expect(editPayload({ ...fieldsOf(holly), commonName: " " })).toEqual({
			error: "Enter the plant's common name.",
		});
	});

	// @spec PLANT-UI-013
	it("shows the notes when the row is expanded", () => {
		expect(plantDetails(holly)).toEqual([
			"Planted 2024-04-01",
			"Notes: Berries in winter",
		]);
		expect(
			plantDetails({ ...holly, notes: null, planting_date: null }),
		).toEqual([]);
	});
});
