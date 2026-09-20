import type { RiskScale, RiskTier } from "../../types/disease";
import { RISK_TIER_COLORS, RISK_TIERS } from "../../types/disease";

interface RiskMeterProps {
	scale: RiskScale;
	score: number;
	tier: RiskTier;
	compact?: boolean;
}

/**
 * Four tier bands laid out on the model's native scale with a marker at the score and
 * the tier thresholds ticked beneath. The active band is drawn at full strength.
 */
export default function RiskMeter({
	scale,
	score,
	tier,
	compact = false,
}: RiskMeterProps) {
	const span = scale.max - scale.min;
	const bounds = [
		scale.min,
		scale.moderate_at,
		scale.high_at,
		scale.severe_at,
		scale.max,
	];
	// Highlight the band the score sits in. Lawn history can raise `tier` above it;
	// the badge and tier note carry that, while the meter stays true to the scale.
	const scoreBand =
		score >= scale.severe_at
			? "Severe"
			: score >= scale.high_at
				? "High"
				: score >= scale.moderate_at
					? "Moderate"
					: "Low";
	const markerPct =
		(Math.min(scale.max, Math.max(scale.min, score)) - scale.min) / span;

	return (
		<div
			role="img"
			aria-label={`Risk meter: ${tier}`}
			style={{ ...styles.wrap, paddingTop: compact ? 6 : 10 }}
		>
			<div style={styles.track}>
				{RISK_TIERS.map((band, i) => (
					<div
						key={band}
						style={{
							...styles.band,
							height: compact ? 6 : 10,
							flexGrow: (bounds[i + 1] - bounds[i]) / span,
							backgroundColor: RISK_TIER_COLORS[band],
							opacity: band === scoreBand ? 1 : 0.25,
						}}
					/>
				))}
			</div>
			<div style={{ ...styles.marker, left: `${markerPct * 100}%` }} />
			{!compact && (
				<div style={styles.ticks}>
					{bounds.slice(1, 4).map((bound) => (
						<span
							key={bound}
							style={{
								...styles.tick,
								left: `${((bound - scale.min) / span) * 100}%`,
							}}
						>
							{bound}
						</span>
					))}
				</div>
			)}
		</div>
	);
}

const styles: Record<string, React.CSSProperties> = {
	wrap: { position: "relative", width: "100%" },
	track: { display: "flex", gap: 2 },
	band: { flexBasis: 0, borderRadius: 3, minWidth: 0 },
	marker: {
		position: "absolute",
		top: 0,
		width: 0,
		height: 0,
		marginLeft: -5,
		borderLeft: "5px solid transparent",
		borderRight: "5px solid transparent",
		borderTop: "7px solid #2d3748",
	},
	ticks: { position: "relative", height: 14, marginTop: 3 },
	tick: {
		position: "absolute",
		transform: "translateX(-50%)",
		fontSize: "0.65rem",
		color: "#718096",
	},
};
