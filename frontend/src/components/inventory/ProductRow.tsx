import type { Product, ProductFacts } from "../../types/inventory";
import {
	AMENDMENT_KIND_LABELS,
	HERBICIDE_TIMING_LABELS,
	PRODUCT_FORM_LABELS,
} from "../../types/inventory";
import {
	CHANGED_FIELD_LABELS,
	PRODUCT_TARGET_LABELS,
} from "../../types/inventoryTargets";
import EditProductForm from "./EditProductForm";
import ProductDetails from "./ProductDetails";
import StockBadge from "./StockBadge";
import { inv } from "./styles";

export interface Suggestion {
	facts: ProductFacts;
	changed: string[];
}

interface Props {
	product: Product;
	expanded: boolean;
	editing: boolean;
	suggestion?: Suggestion;
	busy: "status" | "refresh" | "archive" | "delete" | null;
	onToggle: () => void;
	onStatus: () => void;
	onEdit: (open: boolean) => void;
	onSaved: (p: Product) => void;
	onRefresh: () => void;
	onArchive: () => void;
	onDelete: () => void;
	onDismissSuggestion: () => void;
	onError: (msg: string) => void;
}

export default function ProductRow(props: Props) {
	const { product: p, expanded, editing, suggestion, busy } = props;
	const profile = p.profile;
	const fracLabel = p.frac_classes.map(fracCode).join(", ");

	return (
		<div style={{ ...styles.row, opacity: p.archived ? 0.6 : 1 }}>
			<div style={styles.rowHeader}>
				<button
					type="button"
					onClick={props.onToggle}
					style={styles.toggle}
					aria-expanded={expanded}
				>
					<span style={styles.chevron}>{expanded ? "▾" : "▸"}</span>
					<div style={styles.rowMain}>
						<div style={styles.titleLine}>
							<span style={styles.name}>{p.name}</span>
							{p.brand && <span style={styles.brand}>{p.brand}</span>}
							{p.archived && <span style={styles.archived}>archived</span>}
						</div>
						<div style={styles.metaRow}>
							<span style={styles.meta}>{PRODUCT_FORM_LABELS[p.form]}</span>
							{npk(p) && <span style={styles.meta}>{npk(p)}</span>}
							{fracLabel && <span style={styles.meta}>FRAC {fracLabel}</span>}
							{p.herbicide_timing && (
								<span style={styles.meta}>
									{HERBICIDE_TIMING_LABELS[p.herbicide_timing]}
								</span>
							)}
							{p.amendment_kind && (
								<span style={styles.meta}>
									{AMENDMENT_KIND_LABELS[p.amendment_kind]}
								</span>
							)}
							{p.targets.length > 0 && (
								<span style={styles.meta}>
									{p.targets.map((t) => PRODUCT_TARGET_LABELS[t]).join(", ")}
								</span>
							)}
						</div>
						{!expanded && profile && (
							<div style={styles.summary}>{profile.summary}</div>
						)}
					</div>
				</button>
				<StockBadge
					status={p.stock_status}
					onCycle={props.onStatus}
					busy={busy === "status"}
				/>
			</div>

			{expanded && (
				<div style={styles.body}>
					{suggestion && (
						<div style={inv.infoBox}>
							The refreshed profile suggests changes to{" "}
							{suggestion.changed
								.map((f) => CHANGED_FIELD_LABELS[f] ?? f)
								.join(", ")}
							.{" "}
							<button
								type="button"
								style={inv.secondaryBtn}
								onClick={() => props.onEdit(true)}
							>
								Review in editor
							</button>{" "}
							<button
								type="button"
								style={inv.neutralBtn}
								onClick={props.onDismissSuggestion}
							>
								Ignore
							</button>
						</div>
					)}

					{editing ? (
						<EditProductForm
							product={p}
							initialFacts={suggestion?.facts}
							onSaved={props.onSaved}
							onCancel={() => props.onEdit(false)}
							onError={props.onError}
						/>
					) : (
						<ProductDetails product={p} />
					)}

					{!editing && (
						<div style={styles.footer}>
							<div style={styles.footerMeta}>
								{profile && p.profile_generated_at
									? `Profile generated ${formatDate(p.profile_generated_at)} · ${p.profile_model}`
									: "Entered by hand — no assistant profile"}
							</div>
							<div style={inv.buttonRow}>
								<button
									type="button"
									style={inv.secondaryBtn}
									onClick={() => props.onEdit(true)}
								>
									Edit
								</button>
								<button
									type="button"
									style={inv.secondaryBtn}
									onClick={props.onRefresh}
									disabled={busy === "refresh"}
								>
									{busy === "refresh"
										? "Identifying…"
										: profile
											? "Regenerate profile"
											: "Generate profile"}
								</button>
								<button
									type="button"
									style={inv.neutralBtn}
									onClick={props.onArchive}
									disabled={busy === "archive"}
								>
									{p.archived ? "Unarchive" : "Archive"}
								</button>
								<button
									type="button"
									style={inv.dangerBtn}
									onClick={props.onDelete}
									disabled={busy === "delete"}
								>
									{busy === "delete" ? "Deleting…" : "Delete"}
								</button>
							</div>
						</div>
					)}
				</div>
			)}
		</div>
	);
}

function npk(p: Product): string | null {
	if (
		p.nitrogen_pct == null &&
		p.phosphorus_pct == null &&
		p.potassium_pct == null
	)
		return null;
	return `${p.nitrogen_pct ?? 0}-${p.phosphorus_pct ?? 0}-${p.potassium_pct ?? 0}`;
}

/** "Frac11" → "11", "FracP07" → "P07", "FracM5" → "M5". */
function fracCode(id: string): string {
	return id.replace(/^Frac/, "");
}

function formatDate(iso: string): string {
	try {
		return new Date(iso).toLocaleDateString();
	} catch {
		return iso;
	}
}

const styles: Record<string, React.CSSProperties> = {
	row: {
		backgroundColor: "#fff",
		borderRadius: 8,
		boxShadow: "0 1px 3px rgba(0,0,0,0.08)",
		overflow: "hidden",
	},
	rowHeader: {
		display: "flex",
		alignItems: "flex-start",
		gap: "0.75rem",
		padding: "0.75rem 1rem 0.75rem 0",
	},
	toggle: {
		display: "flex",
		alignItems: "flex-start",
		gap: "0.75rem",
		flex: 1,
		minWidth: 0,
		padding: "0 0 0 1rem",
		backgroundColor: "transparent",
		border: "none",
		cursor: "pointer",
		textAlign: "left",
		fontFamily: "inherit",
	},
	chevron: { fontSize: "0.85rem", color: "#4a5568", marginTop: 4, width: 12 },
	rowMain: { flex: 1, minWidth: 0 },
	titleLine: {
		display: "flex",
		alignItems: "baseline",
		gap: "0.5rem",
		flexWrap: "wrap",
	},
	name: { fontSize: "1.05rem", color: "#1a202c", fontWeight: 600 },
	brand: { fontSize: "0.8rem", color: "#718096" },
	archived: { fontSize: "0.7rem", color: "#a0aec0", fontStyle: "italic" },
	metaRow: { display: "flex", flexWrap: "wrap", gap: "0.5rem", marginTop: 2 },
	meta: { fontSize: "0.75rem", color: "#4a5568" },
	summary: { fontSize: "0.8rem", color: "#4a5568", marginTop: 4 },
	body: { padding: "0.75rem 1rem 1rem", borderTop: "1px solid #edf2f7" },
	footer: {
		display: "flex",
		justifyContent: "space-between",
		alignItems: "center",
		gap: "0.5rem",
		paddingTop: "0.5rem",
		marginTop: "0.5rem",
		borderTop: "1px solid #edf2f7",
		flexWrap: "wrap",
	},
	footerMeta: { fontSize: "0.7rem", color: "#a0aec0" },
};
