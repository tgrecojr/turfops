import { useEffect, useMemo, useState } from "react";
import { listProducts } from "../api/client";
import type { ApplicationType } from "../types";
import type { Product } from "../types/inventory";
import {
	PRODUCT_CATEGORY_LABELS,
	STOCK_STATUS_LABELS,
} from "../types/inventory";
import { categoriesFor } from "../types/inventoryCompat";

const MANUAL = "manual";

interface Props {
	appType: ApplicationType;
	productId: number | null;
	productName: string;
	/** Picked from the shelf (null = "not in inventory"). */
	onPick: (product: Product | null) => void;
	onNameChange: (name: string) => void;
	inputStyle: React.CSSProperties;
	labelStyle: React.CSSProperties;
}

/**
 * The product field of the application form: pick from the shelf (filtered to categories
 * that can be logged under this type) or type a name that is not in the inventory.
 */
export default function ProductPicker({
	appType,
	productId,
	productName,
	onPick,
	onNameChange,
	inputStyle,
	labelStyle,
}: Props) {
	const [products, setProducts] = useState<Product[] | null>(null);
	const [manual, setManual] = useState(productId == null && productName !== "");

	useEffect(() => {
		let cancelled = false;
		listProducts({ includeArchived: true })
			.then((list) => {
				if (!cancelled) setProducts(list);
			})
			.catch(() => {
				if (!cancelled) setProducts([]);
			});
		return () => {
			cancelled = true;
		};
	}, []);

	const options = useMemo(() => {
		if (!products) return [];
		const allowed = categoriesFor(appType);
		return products.filter(
			(p) =>
				p.id === productId || (!p.archived && allowed.includes(p.category)),
		);
	}, [products, appType, productId]);

	// The shelf is empty: behave exactly like the old free-text field.
	if (products !== null && products.length === 0) {
		return (
			<div>
				<label htmlFor="app-product-name" style={labelStyle}>
					Product Name
				</label>
				<input
					id="app-product-name"
					style={inputStyle}
					value={productName}
					onChange={(e) => onNameChange(e.target.value)}
					placeholder={
						appType === "Fungicide" ? "e.g. Heritage G" : "e.g. Milorganite"
					}
				/>
			</div>
		);
	}

	const selectValue =
		productId != null ? String(productId) : manual ? MANUAL : "";

	const handleSelect = (value: string) => {
		if (value === MANUAL) {
			setManual(true);
			onPick(null);
			return;
		}
		setManual(false);
		if (value === "") {
			onPick(null);
			onNameChange("");
			return;
		}
		const picked = options.find((p) => String(p.id) === value) ?? null;
		onPick(picked);
	};

	return (
		<>
			<div>
				<label htmlFor="app-product" style={labelStyle}>
					Product
				</label>
				<select
					id="app-product"
					style={inputStyle}
					value={selectValue}
					onChange={(e) => handleSelect(e.target.value)}
					disabled={products === null}
				>
					<option value="">— none —</option>
					{options.map((p) => (
						<option key={p.id ?? p.name} value={String(p.id)}>
							{optionLabel(p)}
						</option>
					))}
					<option value={MANUAL}>Not in inventory — type a name…</option>
				</select>
			</div>
			{manual && (
				<div>
					<label htmlFor="app-product-name" style={labelStyle}>
						Product Name
					</label>
					<input
						id="app-product-name"
						style={inputStyle}
						value={productName}
						onChange={(e) => onNameChange(e.target.value)}
						placeholder={
							appType === "Fungicide" ? "e.g. Heritage G" : "e.g. Milorganite"
						}
					/>
				</div>
			)}
		</>
	);
}

function optionLabel(p: Product): string {
	const parts = [p.name];
	if (p.brand && !p.name.toLowerCase().includes(p.brand.toLowerCase()))
		parts.push(`(${p.brand})`);
	if (p.category === "Other") parts.push(`· ${PRODUCT_CATEGORY_LABELS.Other}`);
	if (p.archived) parts.push("· archived");
	else if (p.stock_status !== "InStock")
		parts.push(`· ${STOCK_STATUS_LABELS[p.stock_status]}`);
	return parts.join(" ");
}
