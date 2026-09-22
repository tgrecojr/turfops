import { sharedStyles } from "../../styles/shared";
import { IDENTIFICATION_CONFIDENCE_COLORS } from "../../types";
import type { Product } from "../../types/inventory";
import {
	PRODUCT_CATEGORY_LABELS,
	RATE_UNIT_LABELS,
} from "../../types/inventory";
import { inv } from "./styles";

/** Read-only view of a product's facts and assistant profile. */
export default function ProductDetails({ product: p }: { product: Product }) {
	const profile = p.profile;
	const confidenceColor = profile
		? IDENTIFICATION_CONFIDENCE_COLORS[profile.confidence]
		: null;
	return (
		<>
			{profile && profile.confidence !== "High" && (
				<div style={inv.warnBox}>
					⚠ {profile.confidence} identification confidence — verify these facts
					against your label and correct them with Edit.
				</div>
			)}
			{profile && <p style={styles.summaryFull}>{profile.summary}</p>}
			<dl style={styles.dl}>
				<dt style={styles.dt}>Category</dt>
				<dd style={styles.dd}>{PRODUCT_CATEGORY_LABELS[p.category]}</dd>
				{profile && profile.active_ingredients.length > 0 && (
					<>
						<dt style={styles.dt}>Actives</dt>
						<dd style={styles.dd}>
							{profile.active_ingredients
								.map((a) => (a.pct != null ? `${a.name} ${a.pct}%` : a.name))
								.join(", ")}
						</dd>
					</>
				)}
				{p.label_rate_per_1000sqft != null && p.label_rate_unit && (
					<>
						<dt style={styles.dt}>Label rate</dt>
						<dd style={styles.dd}>
							{p.label_rate_per_1000sqft} {RATE_UNIT_LABELS[p.label_rate_unit]}{" "}
							per 1,000 sq ft{" "}
							<span style={styles.caveat}>
								{rateFromAssistant(p)
									? "— suggested by the assistant; verify against your label. Pre-fills the application form only."
									: "— from your label. Pre-fills the application form only."}
							</span>
						</dd>
					</>
				)}
				{profile && profile.mobility !== "NotApplicable" && (
					<>
						<dt style={styles.dt}>Mobility</dt>
						<dd style={styles.dd}>{profile.mobility}</dd>
					</>
				)}
				{profile && profile.nitrogen_release !== "NotApplicable" && (
					<>
						<dt style={styles.dt}>Nitrogen release</dt>
						<dd style={styles.dd}>{profile.nitrogen_release}</dd>
					</>
				)}
				{profile && (
					<>
						<dt style={styles.dt}>Suitable for</dt>
						<dd style={styles.dd}>{profile.suitable_for}</dd>
					</>
				)}
				{p.notes && (
					<>
						<dt style={styles.dt}>Notes</dt>
						<dd style={styles.dd}>{p.notes}</dd>
					</>
				)}
			</dl>
			{profile && profile.cautions.length > 0 && (
				<div style={inv.warnBox}>
					{profile.cautions.map((c) => (
						<div key={c}>⚠ {c}</div>
					))}
				</div>
			)}
			{profile && confidenceColor && (
				<span
					style={{
						...sharedStyles.badge,
						backgroundColor: `${confidenceColor}22`,
						color: confidenceColor,
						borderColor: confidenceColor,
					}}
					title="LLM identification confidence"
				>
					{profile.confidence} confidence
					{profile.identified_name !== p.name
						? ` · identified as ${profile.identified_name}`
						: ""}
				</span>
			)}
		</>
	);
}

const styles: Record<string, React.CSSProperties> = {
	summaryFull: { fontSize: "0.85rem", color: "#2d3748", margin: "0 0 0.75rem" },
	dl: {
		display: "grid",
		gridTemplateColumns: "max-content 1fr",
		gap: "0.25rem 1rem",
		margin: "0 0 0.75rem",
		fontSize: "0.8rem",
	},
	dt: { color: "#718096", fontWeight: 600 },
	dd: { margin: 0, color: "#2d3748" },
	caveat: { color: "#718096", fontStyle: "italic" },
};

/** True when the stored rate is still exactly what the assistant suggested. */
function rateFromAssistant(p: Product): boolean {
	const suggested = p.profile?.suggested_label_rate;
	return (
		suggested != null &&
		suggested.amount === p.label_rate_per_1000sqft &&
		suggested.unit === p.label_rate_unit
	);
}
