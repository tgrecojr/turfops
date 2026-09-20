import type { SoilSeriesDay } from "../../types/timing";

export type SoilRow = SoilSeriesDay & { label: string };

interface SoilSeasonTableProps {
	rows: SoilRow[];
}

const temp = (value: number | null) =>
	value !== null ? `${value.toFixed(1)}°F` : "—";

/** Table view of the soil chart, for readers who cannot use the plot. */
export default function SoilSeasonTable({ rows }: SoilSeasonTableProps) {
	return (
		<details style={styles.details}>
			<summary style={styles.summary}>View as table</summary>
			<div style={styles.scroll}>
				<table style={styles.table}>
					<thead>
						<tr>
							<th style={styles.th}>Day</th>
							<th style={styles.th}>This year</th>
							<th style={styles.th}>Last year</th>
							<th style={styles.th}>Typical</th>
						</tr>
					</thead>
					<tbody>
						{rows.map((row) => (
							<tr key={row.date}>
								<td style={styles.td}>{row.label}</td>
								<td style={styles.td}>
									{row.this_year === null && row.forecast !== null
										? `${temp(row.forecast)} (forecast)`
										: temp(row.this_year)}
								</td>
								<td style={styles.td}>{temp(row.last_year)}</td>
								<td style={styles.td}>{temp(row.typical)}</td>
							</tr>
						))}
					</tbody>
				</table>
			</div>
		</details>
	);
}

const styles: Record<string, React.CSSProperties> = {
	details: { marginTop: 8 },
	summary: { fontSize: "0.75rem", color: "#718096", cursor: "pointer" },
	scroll: { maxHeight: 280, overflowY: "auto", marginTop: 6 },
	table: { width: "100%", borderCollapse: "collapse" },
	th: {
		textAlign: "left",
		fontSize: "0.7rem",
		color: "#718096",
		textTransform: "uppercase",
		padding: "4px 8px",
		borderBottom: "1px solid #e2e8f0",
		position: "sticky",
		top: 0,
		backgroundColor: "#fff",
	},
	td: {
		fontSize: "0.8rem",
		color: "#2d3748",
		padding: "4px 8px",
		borderBottom: "1px solid #edf2f7",
	},
};
