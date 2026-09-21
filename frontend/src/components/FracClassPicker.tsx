import { useEffect, useRef, useState } from "react";
import { getFracClasses } from "../api/client";
import type { FracClassOption } from "../types";

const LOOKUP_DEBOUNCE_MS = 300;

interface Props {
	productName: string;
	value: string[];
	onChange: (classes: string[]) => void;
	/** The entry already has recorded classes (editing): never overwrite them. */
	recorded: boolean;
}

/**
 * FRAC classes of a fungicide. The product name pre-fills the pick when TurfOps knows the
 * product; when it doesn't, say so — an unrecognised fungicide with no class would silently
 * earn no disease protection and no rotation credit.
 */
export default function FracClassPicker({
	productName,
	value,
	onChange,
	recorded,
}: Props) {
	const [options, setOptions] = useState<FracClassOption[]>([]);
	const [matched, setMatched] = useState<string[]>([]);
	const touched = useRef(recorded);
	const onChangeRef = useRef(onChange);
	onChangeRef.current = onChange;

	useEffect(() => {
		let cancelled = false;
		const timer = setTimeout(() => {
			getFracClasses(productName.trim() || undefined)
				.then((lookup) => {
					if (cancelled) return;
					setOptions(lookup.options);
					setMatched(lookup.matched);
					if (!touched.current) onChangeRef.current(lookup.matched);
				})
				.catch(() => {
					// The picker still works by hand; the backend falls back to the name.
				});
		}, LOOKUP_DEBOUNCE_MS);
		return () => {
			cancelled = true;
			clearTimeout(timer);
		};
	}, [productName]);

	const toggle = (id: string) => {
		touched.current = true;
		onChange(
			value.includes(id) ? value.filter((c) => c !== id) : [...value, id],
		);
	};

	const unrecognized =
		productName.trim() !== "" && matched.length === 0 && value.length === 0;

	return (
		<fieldset style={styles.fieldset}>
			<legend style={styles.legend}>FRAC class (from the product label)</legend>
			<div style={styles.grid}>
				{options.map((o) => (
					<label key={o.id} style={styles.option} title={o.examples.join(", ")}>
						<input
							type="checkbox"
							checked={value.includes(o.id)}
							onChange={() => toggle(o.id)}
						/>
						{o.label}
					</label>
				))}
			</div>
			{unrecognized ? (
				<p role="alert" style={styles.warn}>
					⚠ TurfOps doesn't recognize this product. Pick its FRAC class(es) from
					the label — without one, this application can't count toward disease
					protection or fungicide rotation.
				</p>
			) : (
				<p style={styles.hint}>
					{matched.length > 0
						? "Pre-filled from the product name — correct it if the label says otherwise. Premixes have more than one."
						: "Pick every class on the label; premixes have more than one."}
				</p>
			)}
		</fieldset>
	);
}

const styles: Record<string, React.CSSProperties> = {
	fieldset: {
		gridColumn: "1 / -1",
		border: "1px solid #e2e8f0",
		borderRadius: 6,
		padding: "0.6rem 0.8rem",
		margin: 0,
	},
	legend: { fontSize: "0.8rem", fontWeight: 600, color: "#4a5568" },
	grid: {
		display: "grid",
		gridTemplateColumns: "repeat(auto-fill, minmax(210px, 1fr))",
		gap: "2px 12px",
	},
	option: {
		display: "flex",
		alignItems: "center",
		gap: 6,
		fontSize: "0.82rem",
		color: "#2d3748",
	},
	hint: { fontSize: "0.78rem", color: "#718096", margin: "6px 0 0" },
	warn: {
		fontSize: "0.8rem",
		color: "#9c4221",
		backgroundColor: "#fffaf0",
		border: "1px solid #fbd38d",
		borderRadius: 4,
		padding: "6px 8px",
		margin: "6px 0 0",
	},
};
