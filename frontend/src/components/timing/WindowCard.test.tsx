// @vitest-environment jsdom
import { render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { boundary, stat, timingWindow } from "./testFixtures";
import WindowCard from "./WindowCard";

function rowOf(label: string): HTMLElement {
	const head = screen.getByRole("rowheader", { name: label });
	const row = head.closest("tr");
	if (!row) throw new Error(`no row for ${label}`);
	return row;
}

// @spec TIME-UI-003
describe("WindowCard", () => {
	it("shows name, season, headline, badge, conflict, detail, timeline, triggers, confidence and guidance", () => {
		const w = timingWindow("fall_seeding", "Closing", {
			name: "Fall Seeding & Overseeding",
			headline: "Closing now — threshold reached, held 2 of 5 days",
			detail: "5 cm soil cools to 55°F: reached on Oct 1.",
			conflict: "Pre-emergent was applied on Sep 5.",
			guidance: ["Keep the seedbed moist.", "No pre-emergent this fall."],
			opens: boundary({
				label: "5 cm soil cools to 75°F",
				date: "2026-08-15",
				source: "Observed",
				passed: true,
				typical: stat("2026-08-15", {
					earliest: "2026-08-15",
					latest: "2026-09-02",
					sample_count: 9,
					confidence: "High",
				}),
			}),
			closes: boundary({
				label: "5 cm soil cools to 55°F",
				date: "2026-10-01",
				source: "Tentative",
				passed: true,
				days_held: 2,
				typical: stat("2026-10-06", {
					earliest: "2026-09-28",
					latest: "2026-10-15",
					sample_count: 9,
				}),
			}),
		});
		render(<WindowCard window={w} today="2026-10-03" />);

		expect(
			screen.getByRole("heading", {
				name: /Fall Seeding & Overseeding\s*· 2026/,
			}),
		).toBeInTheDocument();
		expect(screen.getByText(w.headline)).toBeInTheDocument();
		// The state badge: symbol + label.
		expect(screen.getByText("△").parentElement).toHaveTextContent("△ Closing");
		expect(screen.getByText(w.conflict as string)).toBeInTheDocument();
		expect(screen.getByText(w.detail)).toBeInTheDocument();
		expect(screen.getByRole("img", { name: /Typically opens/ })).toBeVisible();

		const opens = rowOf("Opens");
		expect(opens).toHaveTextContent("5 cm soil cools to 75°F");
		expect(opens).toHaveTextContent("Aug 15");
		expect(opens).toHaveTextContent("Aug 15 – Sep 2 · 9 yr");
		expect(opens).toHaveTextContent("Aug 15 (observed)");

		const closes = rowOf("Closes");
		expect(within(closes).getByText("Oct 6")).toBeInTheDocument();
		expect(closes).toHaveTextContent("Sep 28 – Oct 15 · 9 yr");
		expect(closes).toHaveTextContent("Oct 1 (tentative, held 2 of 5 days)");
		// Absent boundaries get no row.
		expect(screen.queryByRole("rowheader", { name: "Ideal from" })).toBeNull();

		expect(
			screen.getByText("Typical dates: High confidence"),
		).toBeInTheDocument();
		expect(screen.getByText("Keep the seedbed moist.")).toBeInTheDocument();
		expect(screen.getByText("No pre-emergent this fall.")).toBeInTheDocument();
	});
});

// @spec TIME-UI-004
describe("WindowCard overdue boundary", () => {
	it("reads 'Later than usual — not yet seen' instead of a date", () => {
		const w = timingWindow("fall_pre_emergent", "NotYet", {
			opens: boundary({
				date: null,
				source: "Typical",
				typical: stat("2026-09-07", {
					earliest: "2026-08-28",
					latest: "2026-09-20",
				}),
			}),
		});
		render(<WindowCard window={w} today="2026-09-25" />);
		const opens = rowOf("Opens");
		expect(
			within(opens).getByText("Later than usual — not yet seen"),
		).toBeVisible();
		// The only dates in the row are the typical statistics, never a "this season" one.
		expect(opens).not.toHaveTextContent("(typical)");
	});
});
