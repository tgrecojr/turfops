import { CONFIDENCE_LABELS } from "../../types";
import type { BoundaryView, TimingWindow } from "../../types/timing";
import { DATE_SOURCE_LABELS, formatMonthDay } from "../../types/timing";
import StateBadge from "./StateBadge";
import WindowTimeline from "./WindowTimeline";

interface WindowCardProps {
	window: TimingWindow;
	today: string;
}

function TypicalCell({ boundary }: { boundary: BoundaryView }) {
	if (!boundary.typical) return <>—</>;
	const { median, earliest, latest, sample_count } = boundary.typical;
	const range =
		earliest === latest
			? ""
			: `${formatMonthDay(earliest)} – ${formatMonthDay(latest)} · `;
	return (
		<>
			<div style={styles.nowrap}>{formatMonthDay(median)}</div>
			<div style={styles.sub}>
				{range}
				{sample_count} yr
			</div>
		</>
	);
}

function seasonText(b: BoundaryView): string {
	if (!b.date) return b.typical ? "Later than usual — not yet seen" : "—";
	const held =
		b.source === "Tentative" && b.days_held !== null
			? `, held ${b.days_held} of 5 days`
			: "";
	return `${formatMonthDay(b.date)} (${DATE_SOURCE_LABELS[b.source]}${held})`;
}

/** One timing window: state, headline, typical-window timeline, triggers, guidance. */
export default function WindowCard({ window: w, today }: WindowCardProps) {
	const rows: [string, BoundaryView | null][] = [
		["Opens", w.opens],
		["Ideal from", w.ideal_from],
		["Ideal until", w.ideal_until],
		["Closes", w.closes],
	];
	const confidence = w.opens.typical?.confidence;

	return (
		<section style={styles.card} aria-labelledby={`window-${w.id}`}>
			<div style={styles.header}>
				<div>
					<h2 id={`window-${w.id}`} style={styles.name}>
						{w.name}
						<span style={styles.season}> · {w.season_year}</span>
					</h2>
					<div style={styles.headline}>{w.headline}</div>
				</div>
				<StateBadge state={w.state} size="lg" />
			</div>

			{w.conflict && <div style={styles.conflict}>{w.conflict}</div>}
			{w.detail && <p style={styles.detail}>{w.detail}</p>}

			<WindowTimeline window={w} today={today} />

			<div style={styles.tableWrap}>
				<table style={styles.table}>
					<thead>
						<tr>
							<th style={styles.th} />
							<th style={styles.th}>Trigger</th>
							<th style={styles.th}>Typical</th>
							<th style={styles.th}>This season</th>
						</tr>
					</thead>
					<tbody>
						{rows.map(
							([label, boundary]) =>
								boundary && (
									<tr key={label}>
										<th scope="row" style={styles.rowHead}>
											{label}
										</th>
										<td style={styles.td}>{boundary.label}</td>
										<td style={styles.td}>
											<TypicalCell boundary={boundary} />
										</td>
										<td style={styles.td}>{seasonText(boundary)}</td>
									</tr>
								),
						)}
					</tbody>
				</table>
			</div>
			{confidence && (
				<div style={styles.confidence}>
					Typical dates: {CONFIDENCE_LABELS[confidence]}
				</div>
			)}

			{w.guidance.length > 0 && (
				<ul style={styles.guidance}>
					{w.guidance.map((line) => (
						<li key={line}>{line}</li>
					))}
				</ul>
			)}
		</section>
	);
}

const styles: Record<string, React.CSSProperties> = {
	card: {
		backgroundColor: "#fff",
		borderRadius: 8,
		padding: "1rem 1.25rem",
		boxShadow: "0 1px 3px rgba(0,0,0,0.08)",
	},
	header: {
		display: "flex",
		justifyContent: "space-between",
		alignItems: "flex-start",
		gap: "1rem",
	},
	name: { margin: 0, fontSize: "1.05rem", color: "#1a202c" },
	season: { fontWeight: 400, color: "#718096", fontSize: "0.85rem" },
	headline: {
		fontSize: "0.95rem",
		fontWeight: 600,
		color: "#2d3748",
		marginTop: 4,
	},
	conflict: {
		marginTop: "0.75rem",
		padding: "0.5rem 0.75rem",
		backgroundColor: "#f7fafc",
		border: "1px solid #e2e8f0",
		borderLeft: "3px solid #718096",
		borderRadius: 4,
		fontSize: "0.85rem",
		color: "#2d3748",
		lineHeight: 1.5,
	},
	detail: {
		fontSize: "0.85rem",
		color: "#4a5568",
		lineHeight: 1.5,
		margin: "0.5rem 0 0",
	},
	tableWrap: { overflowX: "auto", marginTop: "0.75rem" },
	table: { width: "100%", borderCollapse: "collapse" },
	th: {
		textAlign: "left",
		fontSize: "0.7rem",
		color: "#718096",
		textTransform: "uppercase",
		padding: "4px 8px",
		borderBottom: "1px solid #e2e8f0",
	},
	rowHead: {
		textAlign: "left",
		fontSize: "0.8rem",
		fontWeight: 600,
		color: "#2d3748",
		padding: "5px 8px",
		borderBottom: "1px solid #edf2f7",
		whiteSpace: "nowrap",
	},
	td: {
		fontSize: "0.8rem",
		color: "#2d3748",
		padding: "5px 8px",
		borderBottom: "1px solid #edf2f7",
	},
	nowrap: { whiteSpace: "nowrap" },
	sub: { fontSize: "0.7rem", color: "#718096", whiteSpace: "nowrap" },
	confidence: { fontSize: "0.7rem", color: "#718096", marginTop: 4 },
	guidance: {
		margin: "0.75rem 0 0",
		paddingLeft: "1.1rem",
		fontSize: "0.82rem",
		color: "#4a5568",
		lineHeight: 1.55,
	},
};
