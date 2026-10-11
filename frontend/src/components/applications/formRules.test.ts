import { describe, expect, it } from "vitest";
import type { Application, ApplicationType } from "../../types";
import {
	formProblem,
	keepsDeletedPlant,
	keepsProductLink,
	plantField,
	saveThenRestock,
} from "./formRules";

function entry(
	type: ApplicationType,
	plant: { id?: number; deletedName?: string } = {},
): Application {
	return {
		id: 9,
		lawn_profile_id: 1,
		application_type: type,
		product_name: null,
		application_date: "2026-04-01",
		rate_per_1000sqft: null,
		coverage_sqft: null,
		notes: null,
		nitrogen_pct: null,
		phosphorus_pct: null,
		potassium_pct: null,
		plant_id: plant.id ?? null,
		plant_name: plant.deletedName ?? null,
		created_at: "2026-04-01T12:00:00Z",
	};
}

const check = {
	plantId: "",
	followUpEnabled: false,
	followUpDate: "",
	date: "2026-05-01",
};

describe("plant field", () => {
	// @spec APP-FORM-002
	it("shows for any type that can target a plant and is required for plant-only types", () => {
		expect(plantField("Aeration")).toEqual({ shown: false, required: false });
		expect(plantField("Fertilizer")).toEqual({ shown: true, required: false });
		expect(plantField("Pruning")).toEqual({ shown: true, required: true });
	});

	// @spec APP-FORM-002
	it("is not required for a deleted plant's entry while its type is unchanged", () => {
		const orphan = entry("Pruning", { deletedName: "Viburnum" });
		expect(keepsDeletedPlant("Pruning", orphan)).toBe(true);
		expect(plantField("Pruning", orphan)).toEqual({
			shown: true,
			required: false,
		});
		expect(keepsDeletedPlant("Mulching", orphan)).toBe(false);
		expect(plantField("Mulching", orphan).required).toBe(true);
		expect(keepsDeletedPlant("Pruning", entry("Pruning", { id: 3 }))).toBe(
			false,
		);
	});
});

describe("blocking a save", () => {
	// @spec APP-FORM-004
	it("needs a plant, a follow-up date when enabled, and a date not before the entry", () => {
		expect(formProblem({ ...check, type: "Pruning" })).toMatch(/plant/);
		expect(
			formProblem({
				...check,
				type: "Pruning",
				initial: entry("Pruning", { deletedName: "Viburnum" }),
			}),
		).toBeNull();
		expect(
			formProblem({ ...check, type: "Fertilizer", followUpEnabled: true }),
		).toMatch(/follow-up date/);
		expect(
			formProblem({
				...check,
				type: "Fertilizer",
				followUpEnabled: true,
				followUpDate: "2026-04-30",
			}),
		).toMatch(/on or after/);
		expect(formProblem({ ...check, type: "Fertilizer" })).toBeNull();
	});
});

describe("shelf link on a type change", () => {
	// @spec APP-FORM-010
	it("stays only when the product's category accepts the new type", () => {
		expect(keepsProductLink("Fertilizer", "Fertilizer")).toBe(true);
		expect(keepsProductLink("Fungicide", "Fertilizer")).toBe(false);
		expect(keepsProductLink("Other", "Fertilizer")).toBe(true);
		expect(keepsProductLink("Fungicide", null)).toBe(true);
	});
});

describe("save then restock", () => {
	// @spec APP-FORM-011
	it("keeps the save when marking the product In stock fails", async () => {
		const failure = new Error("stock update failed");
		const result = await saveThenRestock(
			async () => undefined,
			async () => {
				throw failure;
			},
		);
		expect(result.stockError).toBe(failure);
	});

	it("still fails when the save itself fails", async () => {
		await expect(
			saveThenRestock(async () => {
				throw new Error("409");
			}, null),
		).rejects.toThrow("409");
	});
});
