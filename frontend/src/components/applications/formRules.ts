// The application form's rules, kept free of React so they can be tested: when the plant
// field shows and is required, what blocks a save, when a shelf link survives a type
// change, and how a save and a follow-on restock fail independently.

import type { Application, ApplicationType } from "../../types";
import { canTargetPlant, isPlantRequiredApplicationType } from "../../types";
import type { ProductCategory } from "../../types/inventory";
import { categoriesFor } from "../../types/inventoryCompat";

/** Editing an entry whose plant was deleted, with its type unchanged. */
export function keepsDeletedPlant(
	type: ApplicationType,
	initial?: Application | null,
): boolean {
	return (
		initial != null &&
		initial.plant_id == null &&
		initial.plant_name != null &&
		initial.application_type === type
	);
}

export function plantField(
	type: ApplicationType,
	initial?: Application | null,
): { shown: boolean; required: boolean } {
	// @spec APP-FORM-002
	return {
		shown: canTargetPlant(type),
		required:
			isPlantRequiredApplicationType(type) && !keepsDeletedPlant(type, initial),
	};
}

export interface FormCheck {
	type: ApplicationType;
	plantId: string;
	initial?: Application | null;
	followUpEnabled: boolean;
	followUpDate: string;
	date: string;
}

/** The message that blocks the save, or null. */
export function formProblem(f: FormCheck): string | null {
	// @spec APP-FORM-004
	if (plantField(f.type, f.initial).required && !f.plantId)
		return "Select which plant this action is for.";
	if (f.followUpEnabled && !f.followUpDate)
		return "Pick a follow-up date or uncheck the follow-up option.";
	if (f.followUpEnabled && f.followUpDate < f.date)
		return "Follow-up date must be on or after the application date.";
	return null;
}

/** Whether a linked shelf product of `category` can stay linked under `type`. */
export function keepsProductLink(
	type: ApplicationType,
	category: ProductCategory | null,
): boolean {
	// @spec APP-FORM-010
	return category == null || categoriesFor(type).includes(category);
}

/**
 * Save, then (when asked) mark the product In stock. A failed save throws; a failed
 * restock does not undo the save and is returned on its own.
 */
export async function saveThenRestock(
	save: () => Promise<unknown>,
	restock: (() => Promise<unknown>) | null,
): Promise<{ stockError: unknown | null }> {
	// @spec APP-FORM-011
	await save();
	if (!restock) return { stockError: null };
	try {
		await restock();
		return { stockError: null };
	} catch (e) {
		return { stockError: e };
	}
}
