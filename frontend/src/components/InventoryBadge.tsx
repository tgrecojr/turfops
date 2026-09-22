import type { InventoryStatus } from "../types/inventory";
import {
	INVENTORY_STATE_COLORS,
	INVENTORY_STATE_LABELS,
	INVENTORY_STATE_SYMBOLS,
	STOCK_STATUS_LABELS,
} from "../types/inventory";

interface Props {
	status: InventoryStatus;
}

/** On hand / Low / Need to buy / Partly on hand — symbol + label, hover for the products. */
export default function InventoryBadge({ status }: Props) {
	const color = INVENTORY_STATE_COLORS[status.state];
	const title = [
		...status.matches.map(
			(m) =>
				`${m.need}: ${m.name}${
					m.stock_status === "InStock"
						? ""
						: ` (${STOCK_STATUS_LABELS[m.stock_status].toLowerCase()})`
				}`,
		),
		...status.missing.map((need) => `${need}: not on shelf`),
	].join("\n");
	return (
		<span
			style={{
				...styles.badge,
				backgroundColor: `${color}26`,
				borderColor: color,
			}}
			title={title}
		>
			<span aria-hidden="true">{INVENTORY_STATE_SYMBOLS[status.state]}</span>{" "}
			{INVENTORY_STATE_LABELS[status.state]}
		</span>
	);
}

const styles: Record<string, React.CSSProperties> = {
	badge: {
		display: "inline-block",
		padding: "1px 8px",
		borderRadius: 10,
		fontSize: "0.7rem",
		fontWeight: 600,
		border: "1px solid",
		color: "#2d3748",
		whiteSpace: "nowrap",
	},
};
