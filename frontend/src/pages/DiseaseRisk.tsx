import { useEffect, useState } from "react";
import { Link, useParams } from "react-router";
import { getDiseaseRisk } from "../api/client";
import DiseaseDetail from "../components/disease/DiseaseDetail";
import RiskMeter from "../components/disease/RiskMeter";
import TierBadge from "../components/disease/TierBadge";
import { sharedStyles } from "../styles/shared";
import type { DiseaseRiskResponse } from "../types/disease";

export default function DiseaseRisk() {
	const { slug } = useParams();
	const [data, setData] = useState<DiseaseRiskResponse | null>(null);
	const [error, setError] = useState<string | null>(null);

	useEffect(() => {
		let cancelled = false;
		getDiseaseRisk()
			.then((res) => {
				if (!cancelled) setData(res);
			})
			.catch((e: unknown) => {
				if (!cancelled)
					setError(
						e instanceof Error ? e.message : "Failed to load disease risk",
					);
			});
		return () => {
			cancelled = true;
		};
	}, []);

	if (error) return <div style={sharedStyles.error}>{error}</div>;
	if (!data)
		return <div style={sharedStyles.loading}>Loading disease risk…</div>;

	const selected = slug
		? data.diseases.find((d) => d.slug === slug)
		: undefined;

	return (
		<div>
			<div style={sharedStyles.headerRow}>
				<div>
					{selected && (
						<Link to="/disease-risk" style={styles.back}>
							← All diseases
						</Link>
					)}
					<h1 style={sharedStyles.pageTitle}>
						{selected ? `${selected.name} Risk` : "Disease Risk"}
					</h1>
				</div>
				<span style={styles.station}>{data.station}</span>
			</div>

			{data.data_notes.length > 0 && (
				<div style={styles.notes}>
					{data.data_notes.map((note) => (
						<div key={note}>{note}</div>
					))}
				</div>
			)}

			{slug && !selected && (
				<div style={sharedStyles.empty}>
					No risk data for “{slug}”.{" "}
					<Link to="/disease-risk">Back to all diseases</Link>
				</div>
			)}

			{selected && <DiseaseDetail risk={selected} today={data.today} />}

			{!slug && (
				<>
					<p style={styles.intro}>
						Each disease is scored independently by its own model, then read off
						on a common Low / Moderate / High / Severe scale. The native scores
						are not comparable to each other, so there is no blended score —
						select a disease for its trend, contributing factors and how it is
						calculated.
					</p>
					{data.diseases.length === 0 && (
						<div style={sharedStyles.empty}>
							Not enough recent weather data to score disease risk.
						</div>
					)}
					<div style={styles.list}>
						{data.diseases.map((risk) => (
							<Link
								key={risk.slug}
								to={`/disease-risk/${risk.slug}`}
								style={{ ...sharedStyles.card, ...styles.row }}
							>
								<div style={styles.rowMain}>
									<div style={styles.rowName}>{risk.name}</div>
									<div style={styles.rowModel}>
										{risk.score_label} · {risk.methodology.model_name}
									</div>
								</div>
								<div style={styles.rowMeter}>
									<RiskMeter
										scale={risk.scale}
										score={risk.score}
										tier={risk.tier}
										compact
									/>
								</div>
								<div style={styles.rowTier}>
									<TierBadge tier={risk.tier} />
								</div>
								<span style={styles.chevron} aria-hidden="true">
									›
								</span>
							</Link>
						))}
					</div>
				</>
			)}
		</div>
	);
}

const styles: Record<string, React.CSSProperties> = {
	back: { fontSize: "0.8rem", color: "#3182ce", textDecoration: "none" },
	station: { fontSize: "0.75rem", color: "#718096" },
	notes: {
		padding: "0.5rem 0.75rem",
		backgroundColor: "#ebf8ff",
		border: "1px solid #bee3f8",
		borderRadius: 6,
		fontSize: "0.8rem",
		color: "#2d3748",
		marginBottom: "1rem",
		lineHeight: 1.5,
	},
	intro: {
		fontSize: "0.85rem",
		color: "#4a5568",
		lineHeight: 1.5,
		margin: "0 0 1rem",
		maxWidth: 760,
	},
	list: { display: "flex", flexDirection: "column", gap: "0.5rem" },
	row: {
		display: "flex",
		alignItems: "center",
		gap: "1rem",
		textDecoration: "none",
		color: "inherit",
	},
	rowMain: { flex: "1 1 200px", minWidth: 0 },
	rowName: { fontSize: "1rem", fontWeight: 600, color: "#1a202c" },
	rowModel: { fontSize: "0.78rem", color: "#718096", marginTop: 2 },
	rowMeter: { flex: "1 1 160px", maxWidth: 260 },
	rowTier: { width: 110, textAlign: "right" },
	chevron: { fontSize: "1.4rem", color: "#a0aec0" },
};
