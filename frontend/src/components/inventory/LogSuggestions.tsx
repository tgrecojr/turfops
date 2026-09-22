import { useEffect, useState } from "react";
import { listLoggedProductSuggestions } from "../../api/client";
import { APPLICATION_TYPE_LABELS } from "../../types";
import type { LoggedProductSuggestion } from "../../types/inventory";
import { inv } from "./styles";

interface Props {
	onAdd: (suggestion: LoggedProductSuggestion) => void;
}

/**
 * Product names from the application log that are not on the shelf yet. The page keys
 * this on a version counter so it re-fetches whenever the shelf changes.
 */
export default function LogSuggestions({ onAdd }: Props) {
	const [items, setItems] = useState<LoggedProductSuggestion[]>([]);
	const [dismissed, setDismissed] = useState<Set<string>>(new Set());

	useEffect(() => {
		let cancelled = false;
		listLoggedProductSuggestions()
			.then((list) => {
				if (!cancelled) setItems(list);
			})
			.catch(() => {
				// Suggestions are a convenience; the page works without them.
			});
		return () => {
			cancelled = true;
		};
	}, []);

	const visible = items.filter((s) => !dismissed.has(s.name.toLowerCase()));
	if (visible.length === 0) return null;

	return (
		<section style={styles.box}>
			<h2 style={styles.title}>From your application log</h2>
			<p style={styles.hint}>
				Products you have logged but never added to the shelf.
			</p>
			<ul style={styles.list}>
				{visible.map((s) => (
					<li key={s.name} style={styles.item}>
						<span style={styles.name}>{s.name}</span>
						<span style={styles.meta}>
							{s.uses}× · last {s.last_used} ·{" "}
							{s.application_types
								.map((t) => APPLICATION_TYPE_LABELS[t] ?? t)
								.join(", ")}
						</span>
						<span style={inv.buttonRow}>
							<button
								type="button"
								style={inv.secondaryBtn}
								onClick={() => onAdd(s)}
							>
								Add to shelf
							</button>
							<button
								type="button"
								style={inv.neutralBtn}
								onClick={() =>
									setDismissed((prev) =>
										new Set(prev).add(s.name.toLowerCase()),
									)
								}
							>
								Not now
							</button>
						</span>
					</li>
				))}
			</ul>
		</section>
	);
}

const styles: Record<string, React.CSSProperties> = {
	box: {
		backgroundColor: "#fff",
		borderRadius: 8,
		padding: "0.75rem 1rem",
		marginBottom: "1rem",
		boxShadow: "0 1px 3px rgba(0,0,0,0.08)",
		maxWidth: 960,
	},
	title: { fontSize: "0.9rem", color: "#2d3748", margin: "0 0 2px" },
	hint: { fontSize: "0.75rem", color: "#718096", margin: "0 0 0.5rem" },
	list: { listStyle: "none", margin: 0, padding: 0 },
	item: {
		display: "flex",
		alignItems: "center",
		gap: "0.75rem",
		flexWrap: "wrap",
		padding: "0.4rem 0",
		borderTop: "1px solid #edf2f7",
	},
	name: { fontWeight: 600, fontSize: "0.85rem", color: "#1a202c" },
	meta: { fontSize: "0.75rem", color: "#718096", flex: 1 },
};
