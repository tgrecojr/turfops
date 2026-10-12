import { sharedStyles } from "../../styles/shared";
import type {
	MicronutrientRecommendation,
	NpkRecommendation,
	PhRecommendation,
	SoilTestSummary,
} from "../../types";
import { formatFullDate } from "../../utils/dates";
import { adviceStyles as styles } from "./adviceStyles";
import {
	adviceBasis,
	DEFICIENCY_TITLE,
	needsRetest,
	nothingToApply,
} from "./display";
import LevelBadge from "./LevelBadge";

function Row({
	label,
	children,
}: {
	label: string;
	children: React.ReactNode;
}) {
	return (
		<div style={styles.recRow}>
			<span style={styles.recLabel}>{label}</span>
			<span style={styles.recValue}>{children}</span>
		</div>
	);
}

function BudgetRow({ npk }: { npk: NpkRecommendation }) {
	return (
		<div style={styles.budgetRow}>
			<span style={styles.recLabel}>N Budget Remaining</span>
			<span style={{ ...styles.recValue, fontWeight: 700 }}>
				{npk.remaining_n_budget_lbs_per_1000sqft.toFixed(2)} lbs/1000sqft
			</span>
		</div>
	);
}

function PhCard({ ph }: { ph: PhRecommendation }) {
	return (
		<div style={sharedStyles.card}>
			<h3 style={styles.recTitle}>pH Adjustment</h3>
			<Row label="Current pH">{ph.current_ph.toFixed(1)}</Row>
			<Row label="Target pH">{ph.target_ph.toFixed(1)}</Row>
			<Row label="Amendment">{ph.amendment}</Row>
			<Row label="Rate">
				<strong>{ph.rate_lbs_per_1000sqft.toFixed(0)} lbs/1000 sqft</strong>
			</Row>
			<p style={styles.recExplanation}>{ph.explanation}</p>
		</div>
	);
}

/** The N-P-K advice, or "No fertilizer needed" when it asks for nothing. */
// @spec SOIL-UI-016
function FertilizerCard({ npk }: { npk: NpkRecommendation }) {
	if (nothingToApply(npk)) {
		return (
			<div style={sharedStyles.card}>
				<h3 style={styles.recTitle}>No fertilizer needed</h3>
				<Row label="Phosphorus">
					<LevelBadge level={npk.phosphorus_level} />
				</Row>
				<Row label="Potassium">
					<LevelBadge level={npk.potassium_level} />
				</Row>
				<BudgetRow npk={npk} />
				<p style={styles.recExplanation}>{npk.explanation}</p>
			</div>
		);
	}
	return (
		<div style={sharedStyles.card}>
			<h3 style={styles.recTitle}>Fertilizer Recommendation</h3>
			<Row label="Recommended Ratio">
				<strong style={{ fontSize: "1.1rem" }}>{npk.recommended_ratio}</strong>
			</Row>
			<Row label="N rate">
				{npk.nitrogen_rate_lbs_per_1000sqft.toFixed(2)} lbs/1000sqft
			</Row>
			<Row label="P₂O₅ rate">
				{npk.phosphorus_rate_lbs_per_1000sqft.toFixed(2)} lbs/1000sqft
				<LevelBadge level={npk.phosphorus_level} />
			</Row>
			<Row label="K₂O rate">
				{npk.potassium_rate_lbs_per_1000sqft.toFixed(2)} lbs/1000sqft
				<LevelBadge level={npk.potassium_level} />
			</Row>
			<Row label="Example Product">
				{npk.example_product_ratio} at{" "}
				{npk.product_rate_lbs_per_1000sqft.toFixed(1)} lbs/1000sqft
			</Row>
			<BudgetRow npk={npk} />
			<p style={styles.recExplanation}>{npk.explanation}</p>
		</div>
	);
}

function DeficiencyTable({ rows }: { rows: MicronutrientRecommendation[] }) {
	return (
		<div style={{ ...sharedStyles.card, marginTop: "1rem" }}>
			<h3 style={styles.recTitle}>{DEFICIENCY_TITLE}</h3>
			<table style={sharedStyles.table}>
				<thead>
					<tr>
						<th style={sharedStyles.th}>Nutrient</th>
						<th style={sharedStyles.th}>Current (ppm)</th>
						<th style={sharedStyles.th}>Threshold (ppm)</th>
						<th style={sharedStyles.th}>Status</th>
						<th style={sharedStyles.th}>Suggestion</th>
					</tr>
				</thead>
				<tbody>
					{rows.map((m) => (
						<tr key={m.nutrient}>
							<td style={sharedStyles.td}>{m.nutrient}</td>
							<td style={sharedStyles.td}>{m.current_ppm.toFixed(1)}</td>
							<td style={sharedStyles.td}>{m.threshold_ppm.toFixed(1)}</td>
							<td style={sharedStyles.td}>
								<LevelBadge level={m.level} />
							</td>
							<td style={{ ...sharedStyles.td, fontSize: "0.8rem" }}>
								{m.suggestion}
							</td>
						</tr>
					))}
				</tbody>
			</table>
		</div>
	);
}

/** Which test the advice reads, how old it is, and a retest nudge past 3 years. */
// @spec SOIL-UI-012, SOIL-UI-014, SOIL-UI-015
function AdviceBasis({
	testDate,
	today,
}: {
	testDate: string;
	today: string | null;
}) {
	return (
		<>
			<p style={styles.basis}>
				{today
					? adviceBasis(testDate, today)
					: `Based on your test of ${formatFullDate(testDate)}`}
			</p>
			{today && needsRetest(testDate, today) && (
				<div role="note" style={styles.retest}>
					This test is more than 3 years old. Turf soil is best retested every
					2–3 years; the advice below may no longer describe your soil.
				</div>
			)}
		</>
	);
}

/** The advice from the latest soil test: pH, N-P-K and low nutrients. */
export default function AdvicePanel({
	summary,
	today,
}: {
	summary: SoilTestSummary;
	today: string | null;
}) {
	const {
		ph_recommendation,
		npk_recommendation,
		micronutrient_recommendations,
	} = summary;
	return (
		<div style={{ marginBottom: "1.5rem" }}>
			<h2 style={sharedStyles.sectionTitle}>Recommendations</h2>
			<AdviceBasis testDate={summary.soil_test.test_date} today={today} />
			<div
				style={{
					display: "grid",
					gap: "1rem",
					gridTemplateColumns: "repeat(auto-fit, minmax(320px, 1fr))",
				}}
			>
				{ph_recommendation && <PhCard ph={ph_recommendation} />}
				{npk_recommendation && <FertilizerCard npk={npk_recommendation} />}
			</div>
			{micronutrient_recommendations.length > 0 && (
				<DeficiencyTable rows={micronutrient_recommendations} />
			)}
		</div>
	);
}
