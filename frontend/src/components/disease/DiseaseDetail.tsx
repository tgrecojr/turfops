import { sharedStyles } from "../../styles/shared";
import type { DiseaseRisk } from "../../types/disease";
import {
	FACTOR_STATUS_COLORS,
	FACTOR_STATUS_SYMBOLS,
	formatShortDate,
	RISK_TIER_COLORS,
} from "../../types/disease";
import DailyRiskChart from "./DailyRiskChart";
import RiskMeter from "./RiskMeter";
import TierBadge from "./TierBadge";

interface DiseaseDetailProps {
	risk: DiseaseRisk;
	today: string;
}

export default function DiseaseDetail({ risk, today }: DiseaseDetailProps) {
	const { methodology } = risk;
	const asOf =
		risk.as_of === today ? "today" : `as of ${formatShortDate(risk.as_of)}`;

	return (
		<div>
			<div style={styles.topRow}>
				<div style={{ ...sharedStyles.card, ...styles.scoreCard }}>
					<div style={styles.scoreLabel}>{risk.score_label}</div>
					<TierBadge tier={risk.tier} size="lg" />
					<div style={styles.asOf}>Current risk · {asOf}</div>
					<RiskMeter scale={risk.scale} score={risk.score} tier={risk.tier} />
					<div style={styles.modelName}>{methodology.model_name}</div>
				</div>
				<div style={{ ...sharedStyles.card, ...styles.chartCard }}>
					<h3 style={sharedStyles.sectionTitle}>
						Daily risk — past week and outlook
					</h3>
					<DailyRiskChart daily={risk.daily} scale={risk.scale} today={today} />
				</div>
			</div>

			<div
				style={{
					...sharedStyles.card,
					...styles.summary,
					borderLeftColor: RISK_TIER_COLORS[risk.tier],
				}}
			>
				{risk.summary}
			</div>

			<div style={{ ...sharedStyles.card, marginBottom: "1rem" }}>
				<h3 style={sharedStyles.sectionTitle}>Contributing factors · {asOf}</h3>
				<table style={styles.factorTable}>
					<tbody>
						{risk.factors.map((factor) => (
							<tr key={factor.label}>
								<td style={styles.factorLabel}>{factor.label}</td>
								<td style={styles.factorValue}>{factor.value}</td>
								<td style={styles.factorNote}>
									<span
										role="img"
										aria-label={`${factor.status} for disease`}
										style={{
											...styles.statusDot,
											backgroundColor: `${FACTOR_STATUS_COLORS[factor.status]}26`,
											borderColor: FACTOR_STATUS_COLORS[factor.status],
										}}
									>
										{FACTOR_STATUS_SYMBOLS[factor.status]}
									</span>
									{factor.note}
								</td>
							</tr>
						))}
					</tbody>
				</table>
			</div>

			<details style={{ ...sharedStyles.card, ...styles.methodology }}>
				<summary style={styles.methodologySummary}>
					How this is calculated
				</summary>
				{!methodology.validated && (
					<div style={styles.experimental}>
						⚠ Experimental — this is a decision-support heuristic, not a
						peer-validated forecasting model. Use it alongside scouting.
					</div>
				)}
				<p style={styles.methodologyText}>{methodology.summary}</p>
				<ol style={styles.steps}>
					{methodology.steps.map((step) => (
						<li key={step} style={styles.step}>
							{step}
						</li>
					))}
				</ol>
				<div style={styles.citation}>
					<em>{risk.pathogen}</em> · {methodology.citation}
				</div>
			</details>
		</div>
	);
}

const styles: Record<string, React.CSSProperties> = {
	topRow: {
		display: "flex",
		flexWrap: "wrap",
		gap: "1rem",
		marginBottom: "1rem",
	},
	scoreCard: {
		flex: "1 1 220px",
		maxWidth: 300,
		alignSelf: "flex-start",
		minHeight: 300,
		boxSizing: "border-box",
		display: "flex",
		flexDirection: "column",
		alignItems: "center",
		justifyContent: "center",
		gap: 8,
		textAlign: "center",
	},
	chartCard: { flex: "3 1 420px", minWidth: 0 },
	scoreLabel: { fontSize: "1.6rem", fontWeight: 700, color: "#1a202c" },
	asOf: { fontSize: "0.75rem", color: "#718096" },
	modelName: { fontSize: "0.75rem", color: "#718096" },
	summary: {
		borderLeft: "4px solid",
		marginBottom: "1rem",
		fontSize: "0.9rem",
		lineHeight: 1.5,
		color: "#2d3748",
	},
	factorTable: { width: "100%", borderCollapse: "collapse" },
	factorLabel: {
		padding: "0.5rem 0.5rem 0.5rem 0",
		fontSize: "0.8rem",
		color: "#718096",
		borderBottom: "1px solid #edf2f7",
		width: "26%",
	},
	factorValue: {
		padding: "0.5rem",
		fontSize: "1rem",
		fontWeight: 700,
		color: "#1a202c",
		borderBottom: "1px solid #edf2f7",
		width: "22%",
		whiteSpace: "nowrap",
	},
	factorNote: {
		padding: "0.5rem 0",
		fontSize: "0.8rem",
		color: "#4a5568",
		borderBottom: "1px solid #edf2f7",
	},
	statusDot: {
		display: "inline-block",
		width: 20,
		height: 20,
		lineHeight: "18px",
		textAlign: "center",
		borderRadius: 10,
		border: "1px solid",
		fontSize: "0.7rem",
		color: "#2d3748",
		marginRight: 8,
		boxSizing: "border-box",
	},
	methodology: { marginBottom: "1rem" },
	methodologySummary: {
		cursor: "pointer",
		fontWeight: 600,
		color: "#2d3748",
		fontSize: "0.95rem",
	},
	experimental: {
		marginTop: "0.75rem",
		padding: "0.5rem 0.75rem",
		backgroundColor: "#fefcbf",
		border: "1px solid #ecc94b",
		borderRadius: 6,
		fontSize: "0.8rem",
		color: "#2d3748",
	},
	methodologyText: {
		fontSize: "0.85rem",
		lineHeight: 1.5,
		color: "#2d3748",
		margin: "0.75rem 0 0.5rem",
	},
	steps: { margin: 0, paddingLeft: "1.25rem" },
	step: {
		fontSize: "0.85rem",
		lineHeight: 1.5,
		color: "#4a5568",
		marginBottom: 4,
	},
	citation: { marginTop: "0.75rem", fontSize: "0.75rem", color: "#718096" },
};
