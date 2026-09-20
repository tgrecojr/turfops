import { Link } from "react-router";
import type { TimingResponse } from "../../types/timing";
import { byUrgency } from "../../types/timing";
import StateBadge from "./StateBadge";

interface TimingWidgetProps {
	data: TimingResponse;
}

/** Dashboard card: the primary seeding / pre-emergent windows and where each stands. */
export default function TimingWidget({ data }: TimingWidgetProps) {
	const windows = data.windows
		.filter((w) => w.priority === "Primary")
		.sort(byUrgency);

	return (
		<div style={styles.card}>
			<div style={styles.header}>
				<div style={styles.label}>Seeding & Pre-Emergent</div>
				<Link to="/timing" style={styles.viewAll}>
					View all ›
				</Link>
			</div>
			{data.soil.avg_5day_f !== null && (
				<div style={styles.soil}>
					{data.soil.depth_cm} cm soil, 5-day mean:{" "}
					<strong>{data.soil.avg_5day_f.toFixed(1)}°F</strong>
				</div>
			)}
			{windows.length === 0 && (
				<div style={styles.empty}>No timing windows available.</div>
			)}
			{windows.map((w) => (
				<Link key={w.id} to="/timing" style={styles.row}>
					<span style={styles.main}>
						<span style={styles.name}>{w.name}</span>
						<span style={styles.headline}>{w.headline}</span>
					</span>
					<StateBadge state={w.state} />
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
	soil: { fontSize: "0.78rem", color: "#4a5568", marginBottom: 6 },
	empty: { fontSize: "0.8rem", color: "#a0aec0" },
	row: {
		display: "flex",
		alignItems: "center",
		gap: 8,
		padding: "6px 0",
		borderTop: "1px solid #edf2f7",
		textDecoration: "none",
		color: "inherit",
	},
	main: { flex: 1, minWidth: 0, display: "flex", flexDirection: "column" },
	name: { fontSize: "0.85rem", fontWeight: 600, color: "#2d3748" },
	headline: { fontSize: "0.75rem", color: "#718096" },
};
