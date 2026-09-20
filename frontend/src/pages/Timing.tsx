import { useEffect, useState } from "react";
import { getTimingWindows } from "../api/client";
import SoilSeasonChart from "../components/timing/SoilSeasonChart";
import TimingMethod from "../components/timing/TimingMethod";
import WindowCard from "../components/timing/WindowCard";
import { sharedStyles } from "../styles/shared";
import type { DateStat, TimingResponse } from "../types/timing";
import { byUrgency, formatMonthDay } from "../types/timing";

function freezeText(stat: DateStat | null, early: "p10" | "p90"): string {
	if (!stat) return "Not enough history";
	const edge = early === "p10" ? "as early as" : "as late as";
	return `${edge} ${formatMonthDay(stat[early])} · ${stat.sample_count} yr`;
}

export default function Timing() {
	const [data, setData] = useState<TimingResponse | null>(null);
	const [error, setError] = useState<string | null>(null);

	useEffect(() => {
		let cancelled = false;
		getTimingWindows()
			.then((res) => {
				if (!cancelled) setData(res);
			})
			.catch((e: unknown) => {
				if (!cancelled)
					setError(
						e instanceof Error ? e.message : "Failed to load timing windows",
					);
			});
		return () => {
			cancelled = true;
		};
	}, []);

	if (error) return <div style={sharedStyles.error}>{error}</div>;
	if (!data)
		return <div style={sharedStyles.loading}>Loading timing windows…</div>;

	const { soil, freeze, context } = data;
	const primary = data.windows
		.filter((w) => w.priority === "Primary")
		.sort(byUrgency);
	const secondary = data.windows.filter((w) => w.priority !== "Primary");

	return (
		<div>
			<div style={sharedStyles.headerRow}>
				<h1 style={sharedStyles.pageTitle}>Seeding & Pre-Emergent Timing</h1>
				<span style={styles.station}>{data.station}</span>
			</div>

			{data.data_notes.length > 0 && (
				<div style={styles.notes}>
					{data.data_notes.map((note) => (
						<div key={note}>{note}</div>
					))}
				</div>
			)}

			<div style={styles.tiles}>
				<div style={styles.tile}>
					<div style={styles.tileLabel}>
						{soil.depth_cm} cm soil · 5-day mean
					</div>
					<div style={styles.tileValue}>
						{soil.avg_5day_f !== null ? `${soil.avg_5day_f.toFixed(1)}°F` : "—"}
					</div>
					<div style={styles.tileNote}>
						{soil.as_of ? `as of ${formatMonthDay(soil.as_of)}` : "No data"}
					</div>
				</div>
				<div style={styles.tile}>
					<div style={styles.tileLabel}>Last spring freeze</div>
					<div style={styles.tileValue}>
						{freeze.last_spring
							? formatMonthDay(freeze.last_spring.median)
							: "—"}
					</div>
					<div style={styles.tileNote}>
						{freezeText(freeze.last_spring, "p90")}
					</div>
				</div>
				<div style={styles.tile}>
					<div style={styles.tileLabel}>First fall freeze</div>
					<div style={styles.tileValue}>
						{freeze.first_fall ? formatMonthDay(freeze.first_fall.median) : "—"}
					</div>
					<div style={styles.tileNote}>
						{freeze.first_fall_this_year
							? `this year: ${formatMonthDay(freeze.first_fall_this_year)}`
							: freezeText(freeze.first_fall, "p10")}
					</div>
				</div>
			</div>

			{(context.summary || context.forecast_alerts.length > 0) && (
				<div style={styles.context}>
					{context.summary && <div>{context.summary}</div>}
					{context.forecast_alerts.map((alert) => (
						<div key={alert}>
							<span aria-hidden="true">⚠ </span>
							{alert}
						</div>
					))}
				</div>
			)}

			<div style={styles.cards}>
				{primary.map((w) => (
					<WindowCard key={w.id} window={w} today={data.today} />
				))}
			</div>

			<section style={{ ...sharedStyles.card, ...styles.chartCard }}>
				<h2 style={styles.sectionTitle}>
					Soil temperature at {soil.depth_cm} cm
				</h2>
				<p style={styles.sectionNote}>
					5-day mean. This year against last year and the average of{" "}
					{data.history_years.length} prior years, with this season's
					thresholds.
				</p>
				<SoilSeasonChart series={data.series} today={data.today} />
			</section>

			{secondary.length > 0 && (
				<>
					<h2 style={styles.groupTitle}>Other seeding options</h2>
					<div style={styles.cards}>
						{secondary.map((w) => (
							<WindowCard key={w.id} window={w} today={data.today} />
						))}
					</div>
				</>
			)}

			<TimingMethod historyYears={data.history_years} depthCm={soil.depth_cm} />
		</div>
	);
}

const styles: Record<string, React.CSSProperties> = {
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
	tiles: {
		display: "grid",
		gridTemplateColumns: "repeat(auto-fit, minmax(180px, 1fr))",
		gap: "0.75rem",
		marginBottom: "1rem",
	},
	tile: {
		backgroundColor: "#fff",
		borderRadius: 8,
		padding: "0.75rem 1rem",
		boxShadow: "0 1px 3px rgba(0,0,0,0.08)",
	},
	tileLabel: { fontSize: "0.75rem", color: "#718096" },
	tileValue: {
		fontSize: "1.5rem",
		fontWeight: 600,
		color: "#1a202c",
		lineHeight: 1.3,
	},
	tileNote: { fontSize: "0.75rem", color: "#4a5568" },
	context: {
		fontSize: "0.85rem",
		color: "#2d3748",
		lineHeight: 1.6,
		marginBottom: "1rem",
		maxWidth: 820,
	},
	cards: {
		display: "grid",
		gridTemplateColumns: "repeat(auto-fit, minmax(min(100%, 460px), 1fr))",
		gap: "1rem",
		alignItems: "start",
	},
	chartCard: { margin: "1rem 0" },
	sectionTitle: { margin: 0, fontSize: "1rem", color: "#1a202c" },
	sectionNote: {
		fontSize: "0.8rem",
		color: "#718096",
		margin: "2px 0 0.5rem",
	},
	groupTitle: {
		fontSize: "0.95rem",
		color: "#4a5568",
		margin: "1.25rem 0 0.5rem",
	},
};
