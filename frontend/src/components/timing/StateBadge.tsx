import type { WindowState } from "../../types/timing";
import {
	WINDOW_STATE_COLORS,
	WINDOW_STATE_LABELS,
	WINDOW_STATE_SYMBOLS,
} from "../../types/timing";

interface StateBadgeProps {
	state: WindowState;
	size?: "sm" | "lg";
}

/** Window-state pill: tinted status color behind ink text, always symbol + label. */
export default function StateBadge({ state, size = "sm" }: StateBadgeProps) {
	const color = WINDOW_STATE_COLORS[state];
	return (
		<span
			style={{
				...styles.badge,
				...(size === "lg" ? styles.large : {}),
				backgroundColor: `${color}26`,
				borderColor: color,
			}}
		>
			<span aria-hidden="true">{WINDOW_STATE_SYMBOLS[state]}</span>{" "}
			{WINDOW_STATE_LABELS[state]}
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
