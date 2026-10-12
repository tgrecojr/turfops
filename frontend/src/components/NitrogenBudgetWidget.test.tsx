// @vitest-environment jsdom
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { NitrogenApplication, NitrogenBudget } from "../types";
import NitrogenBudgetWidget from "./NitrogenBudgetWidget";

function budget(
	applied: number,
	applications: NitrogenApplication[] = [],
	percent = (applied / 3.5) * 100,
): NitrogenBudget {
	const target = 3.5;
	return {
		year: 2026,
		target_lbs_per_1000sqft: target,
		applied_lbs_per_1000sqft: applied,
		remaining_lbs_per_1000sqft: Math.max(target - applied, 0),
		percent_of_target: percent,
		applications,
		grass_type_target: {
			grass_type: "TallFescue",
			min_lbs_per_1000sqft: 2,
			max_lbs_per_1000sqft: 4,
			recommended_lbs_per_1000sqft: target,
		},
	};
}

function feeding(
	date: string,
	name: string | null,
	n: number,
): NitrogenApplication {
	return {
		date,
		product_name: name,
		nitrogen_pct: 20,
		rate_per_1000sqft: n * 5,
		n_lbs_per_1000sqft: n,
	};
}

/** The bar's fill: the element sized by percent of target. */
function fill(container: HTMLElement, pct: number): HTMLElement {
	const el = container.querySelector<HTMLElement>(
		`div[style*="width: ${pct}%"]`,
	);
	if (!el) throw new Error(`no bar at ${pct}%`);
	return el;
}

// @spec RULE-UI-002
describe("NitrogenBudgetWidget", () => {
	it("shows applied against target, the grass range, what remains and each feeding", () => {
		const apps = [
			feeding("2026-04-01", "Spring 24-0-6", 0.5),
			feeding("2026-09-05", null, 1),
		];
		render(<NitrogenBudgetWidget data={budget(1.5, apps)} />);
		expect(screen.getByText("1.5")).toBeInTheDocument();
		expect(
			screen.getByText("/ 3.5 lbs N/1k sqft", { exact: false }),
		).toBeInTheDocument();
		expect(screen.getByText(/2\s*-\s*4 lbs\/1k/)).toBeInTheDocument();
		expect(
			screen.getByText("Remaining: 2.0 lbs N/1k sqft"),
		).toBeInTheDocument();
		expect(screen.getByText("2026-04-01")).toBeInTheDocument();
		expect(screen.getByText("Spring 24-0-6")).toBeInTheDocument();
		expect(screen.getByText("+0.50")).toBeInTheDocument();
		expect(screen.getByText("2026-09-05")).toBeInTheDocument();
		expect(screen.getByText("+1.00")).toBeInTheDocument();
		expect(screen.queryByText(/No nitrogen applications/)).toBeNull();
	});

	it("colors the bar green to 80%, amber above 80% and red above 100%", () => {
		const green = render(<NitrogenBudgetWidget data={budget(2.8, [], 80)} />);
		expect(fill(green.container, 80)).toHaveStyle({
			backgroundColor: "#48bb78",
		});
		green.unmount();
		const amber = render(<NitrogenBudgetWidget data={budget(2.835, [], 81)} />);
		expect(fill(amber.container, 81)).toHaveStyle({
			backgroundColor: "#eab308",
		});
		amber.unmount();
		const red = render(<NitrogenBudgetWidget data={budget(4.0, [], 114)} />);
		expect(fill(red.container, 100)).toHaveStyle({
			backgroundColor: "#ef4444",
		});
		expect(screen.getByText(/Over budget by\s+0.5\s+lbs/)).toBeInTheDocument();
		expect(screen.queryByText(/Remaining:/)).toBeNull();
	});

	it("asks for N-P-K when nothing counted", () => {
		render(<NitrogenBudgetWidget data={budget(0)} />);
		expect(
			screen.getByText(
				"No nitrogen applications recorded yet. Add N-P-K data when logging applications.",
			),
		).toBeInTheDocument();
	});
});
