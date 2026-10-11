import { type GaugeConfig, gaugeLevel } from "./gaugeConfigs";

interface GaugeProps {
	label: string;
	value: number | null;
	unit: string;
	min: number;
	max: number;
	thresholds?: GaugeConfig["thresholds"];
	dry?: GaugeConfig["dry"];
}

const BAR_COLORS = { ok: "#48bb78", warning: "#ecc94b", critical: "#fc8181" };
const WORD_COLORS = { ok: "#2d3748", warning: "#975a16", critical: "#c53030" };

export default function Gauge({
	label,
	value,
	unit,
	min,
	max,
	thresholds,
	dry,
}: GaugeProps) {
	const pct =
		value !== null
			? Math.min(100, Math.max(0, ((value - min) / (max - min)) * 100))
			: 0;

	const { level, label: word } = thresholds
		? gaugeLevel({ thresholds, dry }, value)
		: { level: "ok" as const, label: null };
	const barColor = BAR_COLORS[level];

	return (
		<div style={styles.container}>
			<div style={styles.header}>
				<span style={styles.label}>{label}</span>
				<span style={styles.value}>
					{word && (
						<span style={{ ...styles.word, color: WORD_COLORS[level] }}>
							{word}
						</span>
					)}
					{value !== null ? value.toFixed(1) : "--"} {unit}
				</span>
			</div>
			<div style={styles.track}>
				<div
					style={{
						...styles.bar,
						width: `${pct}%`,
						backgroundColor: barColor,
					}}
				/>
			</div>
		</div>
	);
}

const styles: Record<string, React.CSSProperties> = {
	container: { marginBottom: "1rem" },
	header: {
		display: "flex",
		justifyContent: "space-between",
		marginBottom: 4,
		fontSize: "0.85rem",
	},
	label: { color: "#4a5568", fontWeight: 500 },
	value: { color: "#2d3748", fontWeight: 600 },
	word: { marginRight: 6, fontSize: "0.75rem", fontWeight: 700 },
	track: {
		height: 8,
		backgroundColor: "#e2e8f0",
		borderRadius: 4,
		overflow: "hidden",
	},
	bar: {
		height: "100%",
		borderRadius: 4,
		transition: "width 0.4s ease",
	},
};
