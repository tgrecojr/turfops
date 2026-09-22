import { useCallback, useEffect, useState } from "react";
import { Link } from "react-router";
import { getRecommendations, patchRecommendation } from "../api/client";
import InventoryBadge from "../components/InventoryBadge";
import { sharedStyles } from "../styles/shared";
import type { Recommendation } from "../types";
import { categoryLabel, SEVERITY_COLORS, SEVERITY_SYMBOLS } from "../types";
import { STOCK_STATUS_LABELS } from "../types/inventory";

export default function Recommendations() {
	const [allRecs, setAllRecs] = useState<Recommendation[]>([]);
	const [showAnswered, setShowAnswered] = useState(false);
	const [selected, setSelected] = useState<string | null>(null);
	const [error, setError] = useState<string | null>(null);
	const [loading, setLoading] = useState(true);
	const [actionInFlight, setActionInFlight] = useState<string | null>(null);

	const fetchRecs = useCallback(async () => {
		try {
			const data = await getRecommendations(true);
			setAllRecs(data);
			setError(null);
		} catch (e) {
			setError(e instanceof Error ? e.message : "Failed to load");
		} finally {
			setLoading(false);
		}
	}, []);

	useEffect(() => {
		fetchRecs();
	}, [fetchRecs]);

	const recs = allRecs.filter((r) => !r.dismissed && !r.addressed);
	const answered = allRecs.filter((r) => r.dismissed || r.addressed);

	const handleAction = async (
		rec: Recommendation,
		action: "addressed" | "dismissed",
	) => {
		setActionInFlight(rec.id);
		try {
			await patchRecommendation(rec.id, {
				[action]: true,
				severity: rec.severity,
			});
			setAllRecs((prev) =>
				prev.map((r) => (r.id === rec.id ? { ...r, [action]: true } : r)),
			);
			if (selected === rec.id) setSelected(null);
		} catch (e) {
			setError(e instanceof Error ? e.message : "Failed to update");
		} finally {
			setActionInFlight(null);
		}
	};

	const handleRestore = async (id: string) => {
		setActionInFlight(id);
		try {
			await patchRecommendation(id, { dismissed: false, addressed: false });
			setAllRecs((prev) =>
				prev.map((r) =>
					r.id === id ? { ...r, dismissed: false, addressed: false } : r,
				),
			);
		} catch (e) {
			setError(e instanceof Error ? e.message : "Failed to update");
		} finally {
			setActionInFlight(null);
		}
	};

	const selectedRec = recs.find((r) => r.id === selected);

	if (loading) return <div style={sharedStyles.loading}>Loading...</div>;

	return (
		<div>
			<h1 style={sharedStyles.pageTitle}>Recommendations</h1>

			{error && <div style={sharedStyles.error}>{error}</div>}

			{recs.length === 0 ? (
				<div style={styles.emptyGreen}>
					No active recommendations. Your lawn is looking good!
				</div>
			) : (
				<div style={styles.splitView}>
					{/* List */}
					<div style={styles.list}>
						{recs.map((rec) => {
							const color = SEVERITY_COLORS[rec.severity];
							const symbol = SEVERITY_SYMBOLS[rec.severity];
							const isSelected = rec.id === selected;
							const isActioning = actionInFlight === rec.id;

							return (
								// biome-ignore lint/a11y/useSemanticElements: card contains nested action buttons, so it cannot itself be a <button>
								<div
									key={rec.id}
									role="button"
									tabIndex={0}
									style={{
										...styles.listItem,
										borderLeftColor: color,
										backgroundColor: isSelected ? "#ebf8ff" : "#fff",
									}}
									onClick={() => setSelected(rec.id)}
									onKeyDown={(e) => {
										if (e.key === "Enter" || e.key === " ") {
											if (e.key === " ") e.preventDefault();
											setSelected(rec.id);
										}
									}}
								>
									<div style={styles.listHeader}>
										<span
											style={{
												...sharedStyles.badge,
												backgroundColor: color,
												color: "#fff",
												border: "none",
											}}
										>
											{symbol} {rec.severity}
										</span>
										<span style={styles.category}>
											{categoryLabel(rec.category)}
										</span>
										{rec.inventory && <InventoryBadge status={rec.inventory} />}
									</div>
									<div style={styles.listTitle}>{rec.title}</div>
									<div style={styles.listDesc}>{rec.description}</div>
									<div style={styles.actions}>
										<button
											type="button"
											style={styles.addressBtn}
											disabled={isActioning}
											onClick={(e) => {
												e.stopPropagation();
												handleAction(rec, "addressed");
											}}
										>
											{isActioning ? "Updating..." : "Mark Addressed"}
										</button>
										<button
											type="button"
											style={styles.dismissBtn}
											disabled={isActioning}
											onClick={(e) => {
												e.stopPropagation();
												handleAction(rec, "dismissed");
											}}
										>
											Dismiss
										</button>
									</div>
								</div>
							);
						})}
					</div>

					{/* Detail */}
					{selectedRec && (
						<div style={styles.detail}>
							<h2 style={styles.detailTitle}>{selectedRec.title}</h2>
							<span
								style={{
									...sharedStyles.badge,
									backgroundColor: SEVERITY_COLORS[selectedRec.severity],
									color: "#fff",
									border: "none",
									marginBottom: 8,
									display: "inline-block",
								}}
							>
								{SEVERITY_SYMBOLS[selectedRec.severity]} {selectedRec.severity}
							</span>
							<span style={{ ...styles.category, marginLeft: 8 }}>
								{categoryLabel(selectedRec.category)}
							</span>

							<p style={styles.detailDesc}>{selectedRec.description}</p>

							{selectedRec.explanation && (
								<div style={styles.section}>
									<h3 style={sharedStyles.sectionTitle}>Explanation</h3>
									<p style={styles.sectionText}>{selectedRec.explanation}</p>
								</div>
							)}

							{selectedRec.data_points.length > 0 && (
								<div style={styles.section}>
									<h3 style={sharedStyles.sectionTitle}>Data Points</h3>
									<table style={styles.dataTable}>
										<tbody>
											{selectedRec.data_points.map((dp) => (
												<tr key={`${dp.label}-${dp.source}`}>
													<td style={styles.dpLabel}>{dp.label}</td>
													<td style={styles.dpValue}>{dp.value}</td>
													<td style={styles.dpSource}>{dp.source}</td>
												</tr>
											))}
										</tbody>
									</table>
								</div>
							)}

							{selectedRec.suggested_action && (
								<div style={styles.section}>
									<h3 style={sharedStyles.sectionTitle}>Suggested Action</h3>
									<p style={styles.sectionText}>
										{selectedRec.suggested_action}
									</p>
								</div>
							)}

							{selectedRec.inventory && (
								<div style={styles.section}>
									<h3 style={sharedStyles.sectionTitle}>
										Inventory <InventoryBadge status={selectedRec.inventory} />
									</h3>
									<ul style={styles.inventoryList}>
										{selectedRec.inventory.matches.map((m) => (
											<li key={`${m.need}-${m.product_id}`}>
												<Link to="/inventory" style={styles.inventoryLink}>
													{m.name}
												</Link>{" "}
												covers {m.need}
												{m.stock_status !== "InStock" &&
													` — ${STOCK_STATUS_LABELS[m.stock_status].toLowerCase()}`}
											</li>
										))}
										{selectedRec.inventory.missing.map((need) => (
											<li key={need}>
												Nothing on the shelf for {need} —{" "}
												<Link to="/inventory" style={styles.inventoryLink}>
													add it to Inventory
												</Link>{" "}
												once bought
											</li>
										))}
									</ul>
								</div>
							)}
						</div>
					)}
				</div>
			)}

			{answered.length > 0 && (
				<div style={styles.answered}>
					<button
						type="button"
						style={styles.dismissBtn}
						aria-expanded={showAnswered}
						onClick={() => setShowAnswered((v) => !v)}
					>
						{showAnswered ? "Hide" : "Show"} dismissed / addressed (
						{answered.length})
					</button>
					{showAnswered && (
						<>
							<p style={styles.answeredNote}>
								These still apply but are hidden. One comes back on its own if
								it gets more severe, or once the episode it belonged to is over.
							</p>
							{answered.map((rec) => (
								<div key={rec.id} style={styles.answeredItem}>
									<div>
										<div style={styles.listTitle}>
											{SEVERITY_SYMBOLS[rec.severity]} {rec.title}
										</div>
										<div style={styles.category}>
											{rec.severity} ·{" "}
											{rec.addressed ? "Addressed" : "Dismissed"}
										</div>
									</div>
									<button
										type="button"
										style={styles.dismissBtn}
										disabled={actionInFlight === rec.id}
										onClick={() => handleRestore(rec.id)}
									>
										{actionInFlight === rec.id ? "Updating..." : "Restore"}
									</button>
								</div>
							))}
						</>
					)}
				</div>
			)}
		</div>
	);
}

const styles: Record<string, React.CSSProperties> = {
	emptyGreen: {
		color: "#48bb78",
		fontSize: "1rem",
		padding: "2rem",
		backgroundColor: "#fff",
		borderRadius: 8,
		textAlign: "center" as const,
		boxShadow: "0 1px 3px rgba(0,0,0,0.08)",
	},
	splitView: { display: "flex", gap: "1.5rem", alignItems: "flex-start" },
	list: { flex: 1, minWidth: 0 },
	listItem: {
		backgroundColor: "#fff",
		borderRadius: 8,
		padding: "0.8rem 1rem",
		marginBottom: "0.5rem",
		borderLeft: "4px solid",
		boxShadow: "0 1px 3px rgba(0,0,0,0.08)",
		cursor: "pointer",
	},
	listHeader: {
		display: "flex",
		alignItems: "center",
		gap: 8,
		marginBottom: 4,
	},
	category: { fontSize: "0.75rem", color: "#718096" },
	listTitle: {
		fontWeight: 600,
		fontSize: "0.9rem",
		color: "#2d3748",
		marginBottom: 2,
	},
	listDesc: { fontSize: "0.82rem", color: "#4a5568", lineHeight: 1.4 },
	actions: { display: "flex", gap: 8, marginTop: 8 },
	addressBtn: {
		padding: "4px 12px",
		backgroundColor: "#48bb78",
		color: "#fff",
		border: "none",
		borderRadius: 4,
		cursor: "pointer",
		fontSize: "0.75rem",
		fontWeight: 600,
	},
	dismissBtn: {
		padding: "4px 12px",
		backgroundColor: "transparent",
		color: "#718096",
		border: "1px solid #e2e8f0",
		borderRadius: 4,
		cursor: "pointer",
		fontSize: "0.75rem",
	},
	answered: { marginTop: "1.5rem" },
	answeredNote: { fontSize: "0.8rem", color: "#718096", margin: "8px 0" },
	answeredItem: {
		display: "flex",
		justifyContent: "space-between",
		alignItems: "center",
		gap: 12,
		backgroundColor: "#fff",
		borderRadius: 8,
		padding: "0.6rem 1rem",
		marginBottom: "0.4rem",
		boxShadow: "0 1px 3px rgba(0,0,0,0.08)",
	},
	detail: {
		flex: 1,
		backgroundColor: "#fff",
		borderRadius: 8,
		padding: "1.2rem",
		boxShadow: "0 1px 3px rgba(0,0,0,0.08)",
		position: "sticky" as const,
		top: 16,
	},
	detailTitle: { margin: "0 0 8px", fontSize: "1.1rem", color: "#1a202c" },
	detailDesc: { color: "#4a5568", fontSize: "0.9rem", lineHeight: 1.5 },
	section: { marginTop: "1rem" },
	inventoryList: {
		margin: 0,
		paddingLeft: "1.2rem",
		fontSize: "0.85rem",
		color: "#4a5568",
		lineHeight: 1.6,
	},
	inventoryLink: { color: "#3182ce" },
	sectionText: {
		color: "#4a5568",
		fontSize: "0.85rem",
		lineHeight: 1.5,
		margin: 0,
	},
	dataTable: { width: "100%", fontSize: "0.82rem" },
	dpLabel: { padding: "4px 8px 4px 0", color: "#718096", fontWeight: 500 },
	dpValue: { padding: "4px 8px", color: "#2d3748", fontWeight: 600 },
	dpSource: { padding: "4px 0 4px 8px", color: "#a0aec0", fontSize: "0.75rem" },
};
