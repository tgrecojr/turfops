import { Link } from "react-router";
import type { DiseaseRiskResponse } from "../../types/disease";
import TierBadge from "./TierBadge";

interface DiseaseRiskWidgetProps {
	data: DiseaseRiskResponse;
}

/** Dashboard card: every tracked disease with its current tier, linking to detail. */
export default function DiseaseRiskWidget({ data }: DiseaseRiskWidgetProps) {
	return (
		<div style={styles.card}>
			<div style={styles.header}>
				<div style={styles.label}>Disease Risk</div>
				<Link to="/disease-risk" style={styles.viewAll}>
					View all ›
				</Link>
			</div>
			{data.diseases.length === 0 && (
				<div style={styles.empty}>
					{data.data_notes[0] ?? "Not enough recent weather data."}
				</div>
			)}
			{data.diseases.map((risk) => (
				<Link
					key={risk.slug}
					to={`/disease-risk/${risk.slug}`}
					style={styles.row}
				>
					<span style={styles.name}>{risk.name}</span>
					<span style={styles.score}>{risk.score_label}</span>
					<TierBadge tier={risk.tier} />
				</Link>
			))}
		</div>
	);
}

const styles: Record<string, React.CSSProperties> = {
	card: {
		backgroundColor: "#fff",
		borderRadius: 8,
		padding: "1rem",
		boxShadow: "0 1px 3px rgba(0,0,0,0.08)",
	},
	header: {
		display: "flex",
		justifyContent: "space-between",
		alignItems: "center",
		marginBottom: "0.5rem",
	},
	label: { fontSize: "0.85rem", color: "#4a5568", fontWeight: 500 },
	viewAll: { fontSize: "0.75rem", color: "#3182ce", textDecoration: "none" },
	empty: { fontSize: "0.8rem", color: "#a0aec0" },
	row: {
		display: "flex",
		alignItems: "center",
		gap: 8,
		padding: "5px 0",
		borderTop: "1px solid #edf2f7",
		textDecoration: "none",
		color: "inherit",
	},
	name: { flex: 1, fontSize: "0.85rem", fontWeight: 600, color: "#2d3748" },
	score: { fontSize: "0.75rem", color: "#718096" },
};
