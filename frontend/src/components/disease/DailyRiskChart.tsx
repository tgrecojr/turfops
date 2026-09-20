import {
	Bar,
	BarChart,
	CartesianGrid,
	Cell,
	ReferenceLine,
	ResponsiveContainer,
	Tooltip,
	XAxis,
	YAxis,
} from "recharts";
import type { DailyRisk, RiskScale } from "../../types/disease";
import {
	formatShortDate,
	RISK_TIER_COLORS,
	RISK_TIER_SYMBOLS,
	RISK_TIERS,
} from "../../types/disease";

interface DailyRiskChartProps {
	daily: DailyRisk[];
	scale: RiskScale;
	today: string;
}

interface ChartRow extends DailyRisk {
	label: string;
	/** Value clamped to the scale so out-of-range days still draw. */
	plotted: number;
}

const FORECAST_OPACITY = 0.45;

function formatValue(value: number, scale: RiskScale): string {
	const digits = scale.max <= 10 ? 1 : 0;
	const separator = scale.unit === "%" ? "" : " ";
	return `${value.toFixed(digits)}${separator}${scale.unit}`;
}

function dayKind(row: DailyRisk): string {
	if (row.partial) return "Partial day";
	return row.is_forecast ? "Forecast" : "Observed";
}

function RiskTooltip({
	active,
	payload,
	scale,
}: {
	active?: boolean;
	payload?: { payload: ChartRow }[];
	scale: RiskScale;
}) {
	const row = payload?.[0]?.payload;
	if (!active || !row) return null;
	return (
		<div style={styles.tooltip}>
			<div style={styles.tooltipTitle}>
				{row.label} · {dayKind(row)}
			</div>
			<div>{formatValue(row.value, scale)}</div>
			<div>
				<span
					style={{
						...styles.swatch,
						backgroundColor: RISK_TIER_COLORS[row.tier],
					}}
				/>
				{RISK_TIER_SYMBOLS[row.tier]} {row.tier}
			</div>
		</div>
	);
}

export default function DailyRiskChart({
	daily,
	scale,
	today,
}: DailyRiskChartProps) {
	const rows: ChartRow[] = daily.map((d) => ({
		...d,
		label: d.date === today ? "Today" : formatShortDate(d.date),
		plotted: Math.min(scale.max, Math.max(scale.min, d.value)),
	}));
	const thresholds = [
		{ at: scale.moderate_at, label: "Moderate" },
		{ at: scale.high_at, label: "High" },
		{ at: scale.severe_at, label: "Severe" },
	].filter((t) => t.at > scale.min);
	// Tick the scale ends and the Moderate/Severe bounds; High is labeled on its line.
	const yTicks = [
		...new Set([scale.min, scale.moderate_at, scale.severe_at, scale.max]),
	];

	return (
		<div>
			<ResponsiveContainer width="100%" height={220}>
				<BarChart
					data={rows}
					margin={{ top: 10, right: 56, left: 0, bottom: 0 }}
					barCategoryGap={2}
				>
					<CartesianGrid vertical={false} stroke="#edf2f7" />
					<XAxis
						dataKey="label"
						tick={{ fontSize: 11, fill: "#718096" }}
						tickLine={false}
						axisLine={{ stroke: "#e2e8f0" }}
					/>
					<YAxis
						domain={[scale.min, scale.max]}
						ticks={yTicks}
						tick={{ fontSize: 11, fill: "#718096" }}
						tickLine={false}
						axisLine={false}
						width={36}
					/>
					{thresholds.map((t) => (
						<ReferenceLine
							key={t.label}
							y={t.at}
							stroke="#a0aec0"
							strokeDasharray="3 3"
							label={{
								value: t.label,
								position: "right",
								fontSize: 10,
								fill: "#718096",
							}}
						/>
					))}
					<Tooltip
						cursor={{ fill: "#edf2f7" }}
						content={<RiskTooltip scale={scale} />}
					/>
					<Bar
						dataKey="plotted"
						radius={[4, 4, 0, 0]}
						maxBarSize={44}
						isAnimationActive={false}
					>
						{rows.map((row) => (
							<Cell
								key={row.date}
								fill={RISK_TIER_COLORS[row.tier]}
								fillOpacity={row.is_forecast ? FORECAST_OPACITY : 1}
								stroke={row.date === today ? "#2d3748" : undefined}
								strokeWidth={row.date === today ? 2 : 0}
							/>
						))}
					</Bar>
				</BarChart>
			</ResponsiveContainer>

			<div style={styles.legend}>
				{RISK_TIERS.map((tier) => (
					<span key={tier} style={styles.legendItem}>
						<span
							style={{
								...styles.swatch,
								backgroundColor: RISK_TIER_COLORS[tier],
							}}
						/>
						{tier}
					</span>
				))}
				<span style={styles.legendItem}>
					<span
						style={{
							...styles.swatch,
							backgroundColor: "#718096",
							opacity: FORECAST_OPACITY,
						}}
					/>
					Faded = forecast
				</span>
				<span style={styles.legendItem}>
					<span style={{ ...styles.swatch, ...styles.todaySwatch }} />
					Outlined = today
				</span>
			</div>

			<details style={styles.details}>
				<summary style={styles.summary}>View as table</summary>
				<table style={styles.table}>
					<thead>
						<tr>
							<th style={styles.th}>Day</th>
							<th style={styles.th}>Score</th>
							<th style={styles.th}>Tier</th>
							<th style={styles.th}>Source</th>
						</tr>
					</thead>
					<tbody>
						{rows.map((row) => (
							<tr key={row.date}>
								<td style={styles.td}>{row.label}</td>
								<td style={styles.td}>{formatValue(row.value, scale)}</td>
								<td style={styles.td}>
									{RISK_TIER_SYMBOLS[row.tier]} {row.tier}
								</td>
								<td style={styles.td}>{dayKind(row)}</td>
							</tr>
						))}
					</tbody>
				</table>
			</details>
		</div>
	);
}

const styles: Record<string, React.CSSProperties> = {
	tooltip: {
		backgroundColor: "#fff",
		border: "1px solid #e2e8f0",
		borderRadius: 6,
		padding: "6px 10px",
		fontSize: "0.8rem",
		color: "#2d3748",
		boxShadow: "0 2px 6px rgba(0,0,0,0.1)",
	},
	tooltipTitle: { fontWeight: 600, marginBottom: 2 },
	swatch: {
		display: "inline-block",
		width: 10,
		height: 10,
		borderRadius: 2,
		marginRight: 5,
	},
	todaySwatch: {
		backgroundColor: "transparent",
		border: "2px solid #2d3748",
		boxSizing: "border-box",
	},
	legend: {
		display: "flex",
		flexWrap: "wrap",
		gap: "0.25rem 1rem",
		fontSize: "0.75rem",
		color: "#4a5568",
		marginTop: 6,
	},
	legendItem: { display: "inline-flex", alignItems: "center" },
	details: { marginTop: 8 },
	summary: { fontSize: "0.75rem", color: "#718096", cursor: "pointer" },
	table: { width: "100%", borderCollapse: "collapse", marginTop: 6 },
	th: {
		textAlign: "left",
		fontSize: "0.7rem",
		color: "#718096",
		textTransform: "uppercase",
		padding: "4px 8px",
		borderBottom: "1px solid #e2e8f0",
	},
	td: {
		fontSize: "0.8rem",
		color: "#2d3748",
		padding: "4px 8px",
		borderBottom: "1px solid #edf2f7",
	},
};
