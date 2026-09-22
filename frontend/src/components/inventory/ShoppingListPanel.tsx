import { useEffect, useState } from "react";
import { Link } from "react-router";
import { getShoppingList } from "../../api/client";
import { SEVERITY_COLORS, SEVERITY_SYMBOLS } from "../../types";
import type { ShoppingItem } from "../../types/inventory";
import {
	PRODUCT_CATEGORY_LABELS,
	SHOPPING_KIND_LABELS,
	STOCK_STATUS_LABELS,
} from "../../types/inventory";

interface Props {
	/** Compact: top items + a link (dashboard). Full: every item with its reasons. */
	compact?: boolean;
}

const COMPACT_LIMIT = 4;

/**
 * Needs nothing on the shelf covers, and Low / Out products an active recommendation
 * calls for. Keyed by the page on the shelf version so it re-fetches after changes.
 */
export default function ShoppingListPanel({ compact = false }: Props) {
	const [items, setItems] = useState<ShoppingItem[] | null>(null);
	const [error, setError] = useState<string | null>(null);

	useEffect(() => {
		let cancelled = false;
		getShoppingList()
			.then((list) => {
				if (!cancelled) setItems(list.items);
			})
			.catch((e) => {
				if (!cancelled)
					setError(
						e instanceof Error ? e.message : "Shopping list unavailable",
					);
			});
		return () => {
			cancelled = true;
		};
	}, []);

	if (compact && (items === null || items.length === 0)) return null;

	const shown = compact ? (items ?? []).slice(0, COMPACT_LIMIT) : (items ?? []);
	const more = compact ? Math.max(0, (items?.length ?? 0) - COMPACT_LIMIT) : 0;

	return (
		<section style={compact ? styles.cardCompact : styles.card}>
			<div style={styles.header}>
				<div style={styles.title}>Needs buying</div>
				{compact ? (
					<Link to="/inventory" style={styles.link}>
						View all ›
					</Link>
				) : (
					<span style={styles.hint}>
						From active recommendations and Low / Out products they call for
					</span>
				)}
			</div>
			{error && <div style={styles.empty}>{error}</div>}
			{items === null && !error && <div style={styles.empty}>Loading…</div>}
			{items !== null && items.length === 0 && !compact && (
				<div style={styles.empty}>
					Nothing to buy — the shelf covers every active recommendation.
				</div>
			)}
			{shown.map((item) => (
				<Row key={`${item.kind}-${item.label}`} item={item} compact={compact} />
			))}
			{more > 0 && (
				<Link to="/inventory" style={{ ...styles.link, ...styles.moreLink }}>
					{more} more ›
				</Link>
			)}
		</section>
	);
}

function Row({ item, compact }: { item: ShoppingItem; compact: boolean }) {
	const color = SEVERITY_COLORS[item.urgency];
	return (
		<div style={styles.row}>
			<span style={{ ...styles.urgency, color }} title={item.urgency}>
				{SEVERITY_SYMBOLS[item.urgency]}
			</span>
			<span style={styles.main}>
				<span style={styles.label}>
					<span style={styles.kind}>{SHOPPING_KIND_LABELS[item.kind]}</span>{" "}
					{item.label}
					{item.product && item.product.stock_status !== "InStock" && (
						<span style={styles.meta}>
							{" "}
							· {STOCK_STATUS_LABELS[item.product.stock_status].toLowerCase()}
						</span>
					)}
				</span>
				{!compact && (
					<span style={styles.meta}>
						{item.categories
							.map((c) => PRODUCT_CATEGORY_LABELS[c])
							.join(" or ")}{" "}
						· for {item.reasons.join("; ")}
					</span>
				)}
			</span>
		</div>
	);
}

const styles: Record<string, React.CSSProperties> = {
	card: {
		backgroundColor: "#fff",
		borderRadius: 8,
		padding: "1rem",
		boxShadow: "0 1px 3px rgba(0,0,0,0.08)",
		marginBottom: "1rem",
		maxWidth: 960,
	},
	cardCompact: {
		backgroundColor: "#fff",
		borderRadius: 8,
		padding: "1rem",
		boxShadow: "0 1px 3px rgba(0,0,0,0.08)",
	},
	header: {
		display: "flex",
		justifyContent: "space-between",
		alignItems: "baseline",
		gap: "0.5rem",
		marginBottom: "0.25rem",
		flexWrap: "wrap",
	},
	title: { fontSize: "0.85rem", color: "#4a5568", fontWeight: 500 },
	hint: { fontSize: "0.72rem", color: "#a0aec0" },
	link: { fontSize: "0.75rem", color: "#3182ce", textDecoration: "none" },
	moreLink: { display: "block", paddingTop: 6 },
	empty: { fontSize: "0.8rem", color: "#a0aec0", paddingTop: 4 },
	row: {
		display: "flex",
		alignItems: "flex-start",
		gap: 8,
		padding: "6px 0",
		borderTop: "1px solid #edf2f7",
	},
	urgency: {
		fontSize: "0.85rem",
		width: 14,
		textAlign: "center",
		marginTop: 1,
	},
	main: { flex: 1, minWidth: 0, display: "flex", flexDirection: "column" },
	label: { fontSize: "0.85rem", color: "#2d3748", fontWeight: 600 },
	kind: {
		fontSize: "0.65rem",
		textTransform: "uppercase",
		color: "#718096",
		backgroundColor: "#edf2f7",
		padding: "1px 6px",
		borderRadius: 8,
		fontWeight: 600,
	},
	meta: { fontSize: "0.75rem", color: "#718096", fontWeight: 400 },
};
