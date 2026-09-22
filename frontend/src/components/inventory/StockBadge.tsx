import type { StockStatus } from "../../types/inventory";
import {
	STOCK_STATUS_COLORS,
	STOCK_STATUS_LABELS,
	STOCK_STATUS_SYMBOLS,
} from "../../types/inventory";

interface Props {
	status: StockStatus;
	/** Clicking cycles In stock → Low → Out → In stock. */
	onCycle?: () => void;
	busy?: boolean;
}

/** Stock pill: tinted status color behind ink text, always symbol + label. */
export default function StockBadge({ status, onCycle, busy }: Props) {
	const color = STOCK_STATUS_COLORS[status];
	const style = {
		...styles.badge,
		backgroundColor: `${color}26`,
		borderColor: color,
		cursor: onCycle ? "pointer" : "default",
		opacity: busy ? 0.6 : 1,
	};
	const content = (
		<>
			<span aria-hidden="true">{STOCK_STATUS_SYMBOLS[status]}</span>{" "}
			{STOCK_STATUS_LABELS[status]}
		</>
	);
	if (!onCycle) return <span style={style}>{content}</span>;
	return (
		<button
			type="button"
			style={style}
			onClick={(e) => {
				e.stopPropagation();
				onCycle();
			}}
			disabled={busy}
			title="Click to change stock status"
		>
			{content}
		</button>
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
		fontFamily: "inherit",
	},
};
