// The "View as table" disclosure under a chart: the same values as text, for screen
// readers and for anyone who wants the numbers.

export default function ChartTable({
	columns,
	rows,
}: {
	columns: string[];
	rows: (string | number)[][];
}) {
	// @spec UI-CONV-003
	return (
		<details style={styles.details}>
			<summary style={styles.summary}>View as table</summary>
			<div style={styles.scroll}>
				<table style={styles.table}>
					<thead>
						<tr>
							{columns.map((c) => (
								<th key={c} style={styles.th}>
									{c}
								</th>
							))}
						</tr>
					</thead>
					<tbody>
						{rows.map((row) => (
							<tr key={row.join("|")}>
								{row.map((cell, i) => (
									<td key={columns[i]} style={styles.td}>
										{cell}
									</td>
								))}
							</tr>
						))}
					</tbody>
				</table>
			</div>
		</details>
	);
}

const styles: Record<string, React.CSSProperties> = {
	details: { marginTop: 6, fontSize: "0.78rem" },
	summary: { cursor: "pointer", color: "#3182ce" },
	scroll: { maxHeight: 240, overflow: "auto", marginTop: 4 },
	table: { borderCollapse: "collapse", width: "100%" },
	th: {
		textAlign: "left",
		padding: "2px 8px",
		borderBottom: "1px solid #e2e8f0",
		color: "#4a5568",
		position: "sticky",
		top: 0,
		background: "#fff",
	},
	td: {
		padding: "2px 8px",
		borderBottom: "1px solid #f7fafc",
		color: "#2d3748",
	},
};
