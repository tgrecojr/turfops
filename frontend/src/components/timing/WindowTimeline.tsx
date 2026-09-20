import type { TimingWindow } from "../../types/timing";
import { dayNumber, formatMonthDay } from "../../types/timing";

interface WindowTimelineProps {
	window: TimingWindow;
	today: string;
}

const PAD_DAYS = 10;

/**
 * The typical window on a time axis: the historical spread (lightest), the median
 * opens→closes span, the ideal stretch (darkest) and a marker for today.
 */
export default function WindowTimeline({
	window: w,
	today,
}: WindowTimelineProps) {
	const opens = w.opens.typical;
	const closes = w.closes.typical;
	if (!opens || !closes) return null;

	const start = dayNumber(opens.earliest) - PAD_DAYS;
	const end = dayNumber(closes.latest) + PAD_DAYS;
	const span = end - start;
	if (span <= 0) return null;
	const pct = (iso: string) => ((dayNumber(iso) - start) / span) * 100;
	const bar = (from: string, to: string): React.CSSProperties => ({
		left: `${pct(from)}%`,
		width: `${Math.max(pct(to) - pct(from), 0.5)}%`,
	});

	const hasIdeal = w.ideal_from !== null || w.ideal_until !== null;
	const idealFrom = w.ideal_from?.typical?.median ?? opens.median;
	const idealUntil = w.ideal_until?.typical?.median ?? closes.median;
	const todayPct = pct(today);
	const showToday = todayPct >= 0 && todayPct <= 100;

	return (
		<div
			role="img"
			aria-label={`Typically opens ${formatMonthDay(opens.median)} and closes ${formatMonthDay(closes.median)}`}
		>
			<div style={styles.track}>
				<div
					style={{ ...styles.spread, ...bar(opens.earliest, closes.latest) }}
				/>
				<div
					style={{ ...styles.typical, ...bar(opens.median, closes.median) }}
				/>
				{hasIdeal && (
					<div style={{ ...styles.ideal, ...bar(idealFrom, idealUntil) }} />
				)}
				{showToday && (
					<div style={{ ...styles.today, left: `${todayPct}%` }}>
						<span style={styles.todayLabel}>Today</span>
					</div>
				)}
			</div>
			<div style={styles.axis}>
				<span style={{ ...styles.tick, left: `${pct(opens.median)}%` }}>
					{formatMonthDay(opens.median)}
				</span>
				<span style={{ ...styles.tick, left: `${pct(closes.median)}%` }}>
					{formatMonthDay(closes.median)}
				</span>
			</div>
			<div style={styles.legend}>
				<span style={styles.legendItem}>
					<span style={{ ...styles.swatch, backgroundColor: "#d5e4f7" }} />
					Earliest–latest on record
				</span>
				<span style={styles.legendItem}>
					<span style={{ ...styles.swatch, backgroundColor: "#8db5e8" }} />
					Typical window
				</span>
				{hasIdeal && (
					<span style={styles.legendItem}>
						<span style={{ ...styles.swatch, backgroundColor: "#2a78d6" }} />
						Ideal stretch
					</span>
				)}
			</div>
		</div>
	);
}

const styles: Record<string, React.CSSProperties> = {
	track: {
		position: "relative",
		height: 18,
		marginTop: 18,
		backgroundColor: "#f7fafc",
		borderRadius: 4,
	},
	spread: {
		position: "absolute",
		top: 5,
		height: 8,
		backgroundColor: "#d5e4f7",
		borderRadius: 4,
	},
	typical: {
		position: "absolute",
		top: 3,
		height: 12,
		backgroundColor: "#8db5e8",
		borderRadius: 4,
	},
	ideal: {
		position: "absolute",
		top: 1,
		height: 16,
		backgroundColor: "#2a78d6",
		borderRadius: 4,
	},
	today: {
		position: "absolute",
		top: -4,
		bottom: -4,
		width: 2,
		marginLeft: -1,
		backgroundColor: "#1a202c",
	},
	todayLabel: {
		position: "absolute",
		top: -15,
		left: "50%",
		transform: "translateX(-50%)",
		fontSize: "0.65rem",
		fontWeight: 600,
		color: "#1a202c",
		whiteSpace: "nowrap",
	},
	axis: { position: "relative", height: 16, marginTop: 4 },
	tick: {
		position: "absolute",
		transform: "translateX(-50%)",
		fontSize: "0.7rem",
		color: "#4a5568",
		whiteSpace: "nowrap",
	},
	legend: {
		display: "flex",
		flexWrap: "wrap",
		gap: "0.25rem 1rem",
		fontSize: "0.7rem",
		color: "#718096",
		marginTop: 4,
	},
	legendItem: { display: "inline-flex", alignItems: "center" },
	swatch: {
		display: "inline-block",
		width: 10,
		height: 10,
		borderRadius: 2,
		marginRight: 5,
	},
};
