import type { RiskTier } from "../../types/disease";
import { RISK_TIER_COLORS, RISK_TIER_SYMBOLS } from "../../types/disease";

interface TierBadgeProps {
	tier: RiskTier;
	size?: "sm" | "lg";
}

/** Tier pill: tinted status color behind ink text, always with symbol + label. */
export default function TierBadge({ tier, size = "sm" }: TierBadgeProps) {
	const color = RISK_TIER_COLORS[tier];
	return (
		<span
			style={{
				...styles.badge,
				...(size === "lg" ? styles.large : {}),
				backgroundColor: `${color}26`,
				borderColor: color,
			}}
		>
			<span aria-hidden="true">{RISK_TIER_SYMBOLS[tier]}</span> {tier}
		</span>
	);
}

const styles: Record<string, React.CSSProperties> = {
	badge: {
		display: "inline-block",
		padding: "2px 10px",
		borderRadius: 10,
		fontSize: "0.75rem",
		fontWeight: 600,
		border: "1px solid",
		color: "#2d3748",
		whiteSpace: "nowrap",
	},
	large: { fontSize: "0.9rem", padding: "4px 14px", borderRadius: 14 },
};
