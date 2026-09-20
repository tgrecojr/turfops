import { sharedStyles } from "../../styles/shared";
import type { DiseaseManagement, FungicideProgram } from "../../types/disease";
import { formatShortDate, MANAGEMENT_ACTION_LABELS } from "../../types/disease";

interface ManagementPanelProps {
	management: DiseaseManagement;
}

function ProgramCard({
	title,
	program,
}: {
	title: string;
	program: FungicideProgram;
}) {
	return (
		<div style={{ ...sharedStyles.card, ...styles.program }}>
			<h3 style={sharedStyles.sectionTitle}>{title}</h3>
			<div style={styles.meta}>
				<strong>When:</strong> {program.when}
			</div>
			<div style={styles.meta}>
				<strong>Interval:</strong> {program.interval}
			</div>
			<table style={styles.table}>
				<thead>
					<tr>
						<th style={styles.th}>Class</th>
						<th style={styles.th}>Examples</th>
						<th style={styles.th}>Efficacy</th>
					</tr>
				</thead>
				<tbody>
					{program.options.map((option) => (
						<tr
							key={option.frac_class}
							style={option.recommended ? styles.pickRow : undefined}
						>
							<td style={styles.td}>
								<div style={styles.className}>{option.class_label}</div>
								{option.recommended && (
									<span style={{ ...styles.tag, ...styles.pickTag }}>
										★ Suggested next
									</span>
								)}
								{option.last_used && (
									<span style={{ ...styles.tag, ...styles.lastTag }}>
										↻ Used last — rotate
									</span>
								)}
							</td>
							<td style={styles.td}>
								{option.examples.join(", ")}
								{option.note && <div style={styles.note}>{option.note}</div>}
							</td>
							<td style={styles.td}>{option.efficacy}</td>
						</tr>
					))}
				</tbody>
			</table>
			<ul style={styles.list}>
				{program.guidance.map((line) => (
					<li key={line} style={styles.listItem}>
						{line}
					</li>
				))}
			</ul>
		</div>
	);
}

/** "What to do": the action headline, cultural practices, and both spray programs. */
export default function ManagementPanel({ management }: ManagementPanelProps) {
	const { protection } = management;
	return (
		<div>
			<div style={{ ...sharedStyles.card, ...styles.action }}>
				<div style={styles.actionLabel}>
					What to do · {MANAGEMENT_ACTION_LABELS[management.action]}
				</div>
				<div style={styles.headline}>{management.headline}</div>
				{protection && (
					<div style={styles.protection}>
						{protection.product} ({protection.class_label}) · applied{" "}
						{formatShortDate(protection.applied_on)} · covers through about{" "}
						{formatShortDate(protection.protected_through)} (
						{protection.days_remaining} d left)
					</div>
				)}
			</div>

			<div style={{ ...sharedStyles.card, marginBottom: "1rem" }}>
				<h3 style={sharedStyles.sectionTitle}>Cultural practices</h3>
				<ul style={styles.list}>
					{management.cultural.map((line) => (
						<li key={line} style={styles.listItem}>
							{line}
						</li>
					))}
				</ul>
			</div>

			<div style={styles.programs}>
				<ProgramCard
					title="Preventative program"
					program={management.preventative}
				/>
				<ProgramCard
					title="Curative program — if you see symptoms"
					program={management.curative}
				/>
			</div>

			<div style={styles.notes}>
				{management.notes.map((note) => (
					<p key={note} style={styles.noteLine}>
						{note}
					</p>
				))}
			</div>
		</div>
	);
}

const styles: Record<string, React.CSSProperties> = {
	action: { marginBottom: "1rem", borderLeft: "4px solid #2d3748" },
	actionLabel: {
		fontSize: "0.7rem",
		fontWeight: 600,
		color: "#718096",
		textTransform: "uppercase",
		letterSpacing: "0.04em",
		marginBottom: 4,
	},
	headline: {
		fontSize: "1rem",
		fontWeight: 600,
		color: "#1a202c",
		lineHeight: 1.45,
	},
	protection: { marginTop: 6, fontSize: "0.8rem", color: "#4a5568" },
	programs: {
		display: "flex",
		flexWrap: "wrap",
		gap: "1rem",
		marginBottom: "1rem",
	},
	program: { flex: "1 1 380px", minWidth: 0 },
	meta: {
		fontSize: "0.8rem",
		color: "#4a5568",
		marginBottom: 4,
		lineHeight: 1.45,
	},
	table: { width: "100%", borderCollapse: "collapse", margin: "0.5rem 0" },
	th: {
		textAlign: "left",
		fontSize: "0.7rem",
		color: "#718096",
		textTransform: "uppercase",
		padding: "4px 6px",
		borderBottom: "1px solid #e2e8f0",
	},
	td: {
		fontSize: "0.8rem",
		color: "#2d3748",
		padding: "6px",
		borderBottom: "1px solid #edf2f7",
		verticalAlign: "top",
	},
	pickRow: { backgroundColor: "#f7fafc" },
	className: { fontWeight: 600 },
	tag: {
		display: "inline-block",
		marginTop: 3,
		padding: "1px 7px",
		borderRadius: 8,
		fontSize: "0.68rem",
		fontWeight: 600,
		border: "1px solid",
		color: "#2d3748",
		whiteSpace: "nowrap",
	},
	pickTag: { backgroundColor: "#c6f6d5", borderColor: "#48bb78" },
	lastTag: { backgroundColor: "#edf2f7", borderColor: "#a0aec0" },
	note: { fontSize: "0.72rem", color: "#718096", marginTop: 2 },
	list: { margin: 0, paddingLeft: "1.1rem" },
	listItem: {
		fontSize: "0.82rem",
		color: "#4a5568",
		lineHeight: 1.5,
		marginBottom: 3,
	},
	notes: { marginBottom: "1rem" },
	noteLine: {
		fontSize: "0.75rem",
		color: "#718096",
		lineHeight: 1.5,
		margin: "0 0 4px",
	},
};
