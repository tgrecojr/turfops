import { useState } from "react";
import { type UpdateProductBody, updateProduct } from "../../api/client";
import type {
	AmendmentKind,
	HerbicideTiming,
	Product,
	ProductCategory,
	ProductFacts,
	ProductForm,
	ProductTarget,
	RateUnit,
	StockStatus,
} from "../../types/inventory";
import {
	factsOf,
	PRODUCT_CATEGORIES,
	PRODUCT_CATEGORY_LABELS,
	PRODUCT_FORM_LABELS,
	PRODUCT_FORMS,
	RATE_UNIT_LABELS,
	RATE_UNITS,
	STOCK_STATUS_LABELS,
	STOCK_STATUSES,
} from "../../types/inventory";
import { TARGETS_BY_CATEGORY } from "../../types/inventoryTargets";
import CategoryFields from "./CategoryFields";
import { inv } from "./styles";

interface Props {
	product: Product;
	/** Start from these facts instead of the product's own (a refreshed profile's suggestion). */
	initialFacts?: ProductFacts;
	onSaved: (product: Product) => void;
	onCancel: () => void;
	onError: (msg: string) => void;
}

const numOrNull = (s: string): number | null => {
	if (s.trim() === "") return null;
	const n = Number(s);
	return Number.isFinite(n) ? n : null;
};
const str = (n: number | null) => (n == null ? "" : String(n));

/** Every user-owned field. The profile is not editable — regenerate it instead. */
export default function EditProductForm({
	product,
	initialFacts,
	onSaved,
	onCancel,
	onError,
}: Props) {
	const facts = initialFacts ?? factsOf(product);
	const [name, setName] = useState(product.name);
	const [brand, setBrand] = useState(product.brand ?? "");
	const [status, setStatus] = useState<StockStatus>(product.stock_status);
	const [category, setCategory] = useState<ProductCategory>(facts.category);
	const [form, setForm] = useState<ProductForm>(facts.form);
	const [rate, setRate] = useState(str(facts.label_rate_per_1000sqft));
	const [rateUnit, setRateUnit] = useState<RateUnit>(
		facts.label_rate_unit ?? "Lb",
	);
	const [n, setN] = useState(str(facts.nitrogen_pct));
	const [p, setP] = useState(str(facts.phosphorus_pct));
	const [k, setK] = useState(str(facts.potassium_pct));
	const [frac, setFrac] = useState<string[]>(facts.frac_classes);
	const [timing, setTiming] = useState<HerbicideTiming | "">(
		facts.herbicide_timing ?? "",
	);
	const [targets, setTargets] = useState<ProductTarget[]>(facts.targets);
	const [amendment, setAmendment] = useState<AmendmentKind | "">(
		facts.amendment_kind ?? "",
	);
	const [notes, setNotes] = useState(product.notes ?? "");
	const [saving, setSaving] = useState(false);

	const targetOptions = TARGETS_BY_CATEGORY[category];
	const toggleTarget = (t: ProductTarget) =>
		setTargets((prev) =>
			prev.includes(t) ? prev.filter((x) => x !== t) : [...prev, t],
		);

	const handleSubmit = async (e: React.FormEvent) => {
		e.preventDefault();
		if (product.id == null) return;
		const rateVal = numOrNull(rate);
		const body: UpdateProductBody = {
			name: name.trim(),
			brand: brand.trim() || null,
			stock_status: status,
			category,
			form,
			label_rate_per_1000sqft: rateVal,
			label_rate_unit: rateVal == null ? null : rateUnit,
			nitrogen_pct:
				category === "Fertilizer" || category === "Supplement"
					? numOrNull(n)
					: null,
			phosphorus_pct:
				category === "Fertilizer" || category === "Supplement"
					? numOrNull(p)
					: null,
			potassium_pct:
				category === "Fertilizer" || category === "Supplement"
					? numOrNull(k)
					: null,
			frac_classes: category === "Fungicide" ? frac : [],
			herbicide_timing: category === "Herbicide" && timing ? timing : null,
			targets: targets.filter((t) => targetOptions.includes(t)),
			amendment_kind:
				category === "SoilAmendment" && amendment ? amendment : null,
			notes: notes.trim() || null,
			archived: product.archived,
		};
		setSaving(true);
		try {
			onSaved(await updateProduct(product.id, body));
		} catch (err) {
			onError(err instanceof Error ? err.message : "Failed to save product");
		} finally {
			setSaving(false);
		}
	};

	return (
		<form onSubmit={handleSubmit} style={{ ...inv.form, marginBottom: 0 }}>
			<div style={inv.formGrid}>
				<div>
					<label style={inv.label} htmlFor={`edit-name-${product.id}`}>
						Name
					</label>
					<input
						id={`edit-name-${product.id}`}
						style={inv.input}
						value={name}
						onChange={(e) => setName(e.target.value)}
						required
					/>
				</div>
				<div>
					<label style={inv.label} htmlFor={`edit-brand-${product.id}`}>
						Brand
					</label>
					<input
						id={`edit-brand-${product.id}`}
						style={inv.input}
						value={brand}
						onChange={(e) => setBrand(e.target.value)}
					/>
				</div>
				<div>
					<label style={inv.label} htmlFor={`edit-category-${product.id}`}>
						Category
					</label>
					<select
						id={`edit-category-${product.id}`}
						style={inv.input}
						value={category}
						onChange={(e) => setCategory(e.target.value as ProductCategory)}
					>
						{PRODUCT_CATEGORIES.map((c) => (
							<option key={c} value={c}>
								{PRODUCT_CATEGORY_LABELS[c]}
							</option>
						))}
					</select>
				</div>
				<div>
					<label style={inv.label} htmlFor={`edit-form-${product.id}`}>
						Form
					</label>
					<select
						id={`edit-form-${product.id}`}
						style={inv.input}
						value={form}
						onChange={(e) => setForm(e.target.value as ProductForm)}
					>
						{PRODUCT_FORMS.map((f) => (
							<option key={f} value={f}>
								{PRODUCT_FORM_LABELS[f]}
							</option>
						))}
					</select>
				</div>
				<div>
					<label style={inv.label} htmlFor={`edit-status-${product.id}`}>
						Stock
					</label>
					<select
						id={`edit-status-${product.id}`}
						style={inv.input}
						value={status}
						onChange={(e) => setStatus(e.target.value as StockStatus)}
					>
						{STOCK_STATUSES.map((s) => (
							<option key={s} value={s}>
								{STOCK_STATUS_LABELS[s]}
							</option>
						))}
					</select>
				</div>
				<div>
					<label style={inv.label} htmlFor={`edit-rate-${product.id}`}>
						Label rate per 1,000 sq ft (optional)
					</label>
					<div style={{ display: "flex", gap: "0.4rem" }}>
						<input
							id={`edit-rate-${product.id}`}
							style={inv.input}
							type="number"
							min="0"
							step="any"
							value={rate}
							onChange={(e) => setRate(e.target.value)}
						/>
						<select
							aria-label="Rate unit"
							style={{ ...inv.input, width: "auto" }}
							value={rateUnit}
							onChange={(e) => setRateUnit(e.target.value as RateUnit)}
						>
							{RATE_UNITS.map((u) => (
								<option key={u} value={u}>
									{RATE_UNIT_LABELS[u]}
								</option>
							))}
						</select>
					</div>
				</div>
				<CategoryFields
					idPrefix={`edit-${product.id}`}
					category={category}
					name={name}
					npk={{ n, p, k }}
					onNpk={(field, v) =>
						field === "n" ? setN(v) : field === "p" ? setP(v) : setK(v)
					}
					timing={timing}
					onTiming={setTiming}
					amendment={amendment}
					onAmendment={setAmendment}
					frac={frac}
					fracRecorded={facts.frac_classes.length > 0}
					onFrac={setFrac}
					targets={targets}
					onToggleTarget={toggleTarget}
				/>
				<div style={inv.full}>
					<label style={inv.label} htmlFor={`edit-notes-${product.id}`}>
						Notes
					</label>
					<input
						id={`edit-notes-${product.id}`}
						style={inv.input}
						value={notes}
						onChange={(e) => setNotes(e.target.value)}
					/>
				</div>
			</div>
			<div style={inv.buttonRow}>
				<button type="submit" style={inv.primaryBtn} disabled={saving}>
					{saving ? "Saving…" : "Save"}
				</button>
				<button type="button" style={inv.neutralBtn} onClick={onCancel}>
					Cancel
				</button>
			</div>
		</form>
	);
}
