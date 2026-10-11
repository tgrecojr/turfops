import { useState } from "react";
import { updatePlant } from "../../api/client";
import { describeError } from "../../api/errors";
import type { Plant, PlantType } from "../../types";
import { PLANT_TYPE_LABELS } from "../../types";
import {
	editPayload,
	fieldsOf,
	PLANT_TYPES,
	type PlantFields,
} from "./plantEdit";

type Styles = Record<
	"form" | "formGrid" | "formLabel" | "input" | "submitBtn" | "secondaryBtn",
	React.CSSProperties
>;

/**
 * Edit a plant's own details. Saving never regenerates its care plan (that is
 * "Regenerate plan").
 */
// @spec PLANT-UI-012
export default function EditPlantForm({
	plant,
	styles,
	onSaved,
	onCancel,
	onError,
}: {
	plant: Plant;
	styles: Styles;
	onSaved: (updated: Plant) => void;
	onCancel: () => void;
	onError: (message: string) => void;
}) {
	const [fields, setFields] = useState<PlantFields>(fieldsOf(plant));
	const [saving, setSaving] = useState(false);
	const set =
		(key: keyof PlantFields) =>
		(e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement>) =>
			setFields((f) => ({ ...f, [key]: e.target.value }));

	const handleSubmit = async (e: React.FormEvent) => {
		e.preventDefault();
		if (plant.id == null) return;
		const result = editPayload(fields);
		if ("error" in result) {
			onError(result.error);
			return;
		}
		setSaving(true);
		try {
			onSaved(await updatePlant(plant.id, result.payload));
		} catch (err) {
			onError(describeError(err));
		} finally {
			setSaving(false);
		}
	};

	const id = (name: string) => `edit-plant-${plant.id}-${name}`;
	return (
		<form onSubmit={handleSubmit} style={styles.form}>
			<div style={styles.formGrid}>
				<div>
					<label style={styles.formLabel} htmlFor={id("common")}>
						Common name
					</label>
					<input
						id={id("common")}
						style={styles.input}
						value={fields.commonName}
						onChange={set("commonName")}
						required
					/>
				</div>
				<div>
					<label style={styles.formLabel} htmlFor={id("scientific")}>
						Scientific name (optional)
					</label>
					<input
						id={id("scientific")}
						style={styles.input}
						value={fields.scientificName}
						onChange={set("scientificName")}
					/>
				</div>
				<div>
					<label style={styles.formLabel} htmlFor={id("type")}>
						Plant type
					</label>
					<select
						id={id("type")}
						style={styles.input}
						value={fields.plantType}
						onChange={(e) =>
							setFields((f) => ({
								...f,
								plantType: e.target.value as PlantType,
							}))
						}
					>
						{PLANT_TYPES.map((t) => (
							<option key={t} value={t}>
								{PLANT_TYPE_LABELS[t]}
							</option>
						))}
					</select>
				</div>
				<div>
					<label style={styles.formLabel} htmlFor={id("location")}>
						Location (optional)
					</label>
					<input
						id={id("location")}
						style={styles.input}
						value={fields.location}
						onChange={set("location")}
					/>
				</div>
				<div>
					<label style={styles.formLabel} htmlFor={id("planted")}>
						Planting date (optional)
					</label>
					<input
						id={id("planted")}
						type="date"
						style={styles.input}
						value={fields.plantingDate}
						onChange={set("plantingDate")}
					/>
				</div>
				<div style={{ gridColumn: "1 / -1" }}>
					<label style={styles.formLabel} htmlFor={id("notes")}>
						Notes (optional)
					</label>
					<input
						id={id("notes")}
						style={styles.input}
						value={fields.notes}
						onChange={set("notes")}
					/>
				</div>
			</div>
			<button type="submit" style={styles.submitBtn} disabled={saving}>
				{saving ? "Saving…" : "Save changes"}
			</button>{" "}
			<button type="button" style={styles.secondaryBtn} onClick={onCancel}>
				Cancel
			</button>
		</form>
	);
}
