import { useState } from "react";
import { type CreateProductBody, createProduct } from "../../api/client";
import type {
	ProductCategory,
	ProductCreated,
	ProductForm,
	StockStatus,
} from "../../types/inventory";
import {
	PRODUCT_CATEGORIES,
	PRODUCT_CATEGORY_LABELS,
	PRODUCT_FORM_LABELS,
	PRODUCT_FORMS,
	STOCK_STATUS_LABELS,
	STOCK_STATUSES,
} from "../../types/inventory";
import { inv } from "./styles";

const ASSISTANT_DECIDES = "";

interface Props {
	onCreated: (created: ProductCreated) => void;
	onError: (msg: string) => void;
}

/**
 * One free-text name; the assistant fills in the rest when it is on. With it off (or
 * unavailable) the category is required and the product is saved by hand.
 */
export default function AddProductForm({ onCreated, onError }: Props) {
	const [name, setName] = useState("");
	const [brand, setBrand] = useState("");
	const [category, setCategory] = useState<ProductCategory | "">(
		ASSISTANT_DECIDES,
	);
	const [form, setForm] = useState<ProductForm | "">("");
	const [status, setStatus] = useState<StockStatus>("InStock");
	const [notes, setNotes] = useState("");
	const [assist, setAssist] = useState(true);
	const [submitting, setSubmitting] = useState(false);

	const handleSubmit = async (e: React.FormEvent) => {
		e.preventDefault();
		if (!assist && category === "") {
			onError("Pick a category to save a product without the assistant.");
			return;
		}
		setSubmitting(true);
		try {
			const body: CreateProductBody = {
				name: name.trim(),
				brand: brand.trim() || undefined,
				category: category || undefined,
				form: form || undefined,
				stock_status: status,
				notes: notes.trim() || undefined,
				assist,
			};
			onCreated(await createProduct(body));
		} catch (err) {
			onError(err instanceof Error ? err.message : "Failed to add product");
		} finally {
			setSubmitting(false);
		}
	};

	return (
		<form onSubmit={handleSubmit} style={inv.form}>
			<div style={inv.formGrid}>
				<div style={inv.full}>
					<label style={inv.label} htmlFor="product-name">
						Product name (as printed on the bag or bottle)
					</label>
					<input
						id="product-name"
						style={inv.input}
						value={name}
						onChange={(e) => setName(e.target.value)}
						placeholder="e.g. Heritage G, Turf Builder 32-0-4, Dimension 0.25G"
						required
					/>
				</div>
				<div>
					<label style={inv.label} htmlFor="product-brand">
						Brand (optional)
					</label>
					<input
						id="product-brand"
						style={inv.input}
						value={brand}
						onChange={(e) => setBrand(e.target.value)}
						placeholder="e.g. Syngenta"
					/>
				</div>
				<div>
					<label style={inv.label} htmlFor="product-category">
						Category{assist ? "" : " (required)"}
					</label>
					<select
						id="product-category"
						style={inv.input}
						value={category}
						onChange={(e) =>
							setCategory(e.target.value as ProductCategory | "")
						}
						required={!assist}
					>
						<option value={ASSISTANT_DECIDES}>
							{assist ? "Let the assistant decide" : "— pick one —"}
						</option>
						{PRODUCT_CATEGORIES.map((c) => (
							<option key={c} value={c}>
								{PRODUCT_CATEGORY_LABELS[c]}
							</option>
						))}
					</select>
				</div>
				<div>
					<label style={inv.label} htmlFor="product-form">
						Form
					</label>
					<select
						id="product-form"
						style={inv.input}
						value={form}
						onChange={(e) => setForm(e.target.value as ProductForm | "")}
					>
						<option value="">
							{assist ? "Let the assistant decide" : "Unknown"}
						</option>
						{PRODUCT_FORMS.map((f) => (
							<option key={f} value={f}>
								{PRODUCT_FORM_LABELS[f]}
							</option>
						))}
					</select>
				</div>
				<div>
					<label style={inv.label} htmlFor="product-status">
						Stock
					</label>
					<select
						id="product-status"
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
				<div style={inv.full}>
					<label style={inv.label} htmlFor="product-notes">
						Notes (optional)
					</label>
					<input
						id="product-notes"
						style={inv.input}
						value={notes}
						onChange={(e) => setNotes(e.target.value)}
						placeholder="e.g. half a bag left in the shed"
					/>
				</div>
				<label style={{ ...inv.checkRow, ...inv.full }}>
					<input
						type="checkbox"
						checked={assist}
						onChange={(e) => setAssist(e.target.checked)}
					/>
					Identify with the assistant (actives, N-P-K, FRAC class, targets)
				</label>
			</div>
			<button type="submit" style={inv.primaryBtn} disabled={submitting}>
				{submitting
					? assist
						? "Identifying…"
						: "Saving…"
					: assist
						? "Add & identify"
						: "Add product"}
			</button>
			<div style={inv.hint}>
				{assist
					? "Identification calls the configured LLM and may take 10–30 seconds. Every fact it returns stays editable — verify against your label."
					: "Saved as entered; you can generate a profile later."}
			</div>
		</form>
	);
}
