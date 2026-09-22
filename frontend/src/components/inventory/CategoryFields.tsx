import type {
	AmendmentKind,
	HerbicideTiming,
	ProductCategory,
	ProductTarget,
} from "../../types/inventory";
import {
	AMENDMENT_KIND_LABELS,
	AMENDMENT_KINDS,
	HERBICIDE_TIMING_LABELS,
	HERBICIDE_TIMINGS,
} from "../../types/inventory";
import {
	PRODUCT_TARGET_LABELS,
	TARGETS_BY_CATEGORY,
} from "../../types/inventoryTargets";
import FracClassPicker from "../FracClassPicker";
import { inv } from "./styles";

interface Props {
	idPrefix: string;
	category: ProductCategory;
	name: string;
	npk: { n: string; p: string; k: string };
	onNpk: (field: "n" | "p" | "k", value: string) => void;
	timing: HerbicideTiming | "";
	onTiming: (t: HerbicideTiming | "") => void;
	amendment: AmendmentKind | "";
	onAmendment: (a: AmendmentKind | "") => void;
	frac: string[];
	fracRecorded: boolean;
	onFrac: (classes: string[]) => void;
	targets: ProductTarget[];
	onToggleTarget: (t: ProductTarget) => void;
}

/** The facts that only make sense for some categories (N-P-K, FRAC, timing, targets…). */
export default function CategoryFields({
	idPrefix,
	category,
	name,
	npk,
	onNpk,
	timing,
	onTiming,
	amendment,
	onAmendment,
	frac,
	fracRecorded,
	onFrac,
	targets,
	onToggleTarget,
}: Props) {
	const targetOptions = TARGETS_BY_CATEGORY[category];
	return (
		<>
			{(category === "Fertilizer" || category === "Supplement") && (
				<div>
					<span style={inv.label}>N-P-K (%)</span>
					<div style={{ display: "flex", gap: "0.4rem" }}>
						{(["n", "p", "k"] as const).map((field) => (
							<input
								key={field}
								aria-label={`${field.toUpperCase()} percent`}
								style={inv.input}
								type="number"
								min="0"
								max="100"
								step="any"
								placeholder={field.toUpperCase()}
								value={npk[field]}
								onChange={(e) => onNpk(field, e.target.value)}
							/>
						))}
					</div>
				</div>
			)}
			{category === "Herbicide" && (
				<div>
					<label style={inv.label} htmlFor={`${idPrefix}-timing`}>
						Herbicide timing
					</label>
					<select
						id={`${idPrefix}-timing`}
						style={inv.input}
						value={timing}
						onChange={(e) => onTiming(e.target.value as HerbicideTiming | "")}
					>
						<option value="">Unknown</option>
						{HERBICIDE_TIMINGS.map((t) => (
							<option key={t} value={t}>
								{HERBICIDE_TIMING_LABELS[t]}
							</option>
						))}
					</select>
				</div>
			)}
			{category === "SoilAmendment" && (
				<div>
					<label style={inv.label} htmlFor={`${idPrefix}-amend`}>
						Amendment kind
					</label>
					<select
						id={`${idPrefix}-amend`}
						style={inv.input}
						value={amendment}
						onChange={(e) => onAmendment(e.target.value as AmendmentKind | "")}
					>
						<option value="">Unknown</option>
						{AMENDMENT_KINDS.map((a) => (
							<option key={a} value={a}>
								{AMENDMENT_KIND_LABELS[a]}
							</option>
						))}
					</select>
				</div>
			)}
			{category === "Fungicide" && (
				<div style={inv.full}>
					<FracClassPicker
						productName={name}
						value={frac}
						onChange={onFrac}
						recorded={fracRecorded}
					/>
				</div>
			)}
			{targetOptions.length > 0 && (
				<fieldset style={{ ...inv.fieldset, ...inv.full }}>
					<legend style={inv.legend}>
						{category === "Seed" ? "Species in the mix" : "Targets"}
					</legend>
					<div style={inv.checkGrid}>
						{targetOptions.map((t) => (
							<label key={t} style={inv.checkRow}>
								<input
									type="checkbox"
									checked={targets.includes(t)}
									onChange={() => onToggleTarget(t)}
								/>
								{PRODUCT_TARGET_LABELS[t]}
							</label>
						))}
					</div>
				</fieldset>
			)}
		</>
	);
}
