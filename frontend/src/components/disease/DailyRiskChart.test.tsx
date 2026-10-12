// @vitest-environment jsdom
import { render, screen, within } from "@testing-library/react";
import { cloneElement, type ReactElement } from "react";
import { describe, expect, it, vi } from "vitest";
import { RISK_TIER_COLORS } from "../../types/disease";
import DailyRiskChart, { RiskTooltip } from "./DailyRiskChart";
import { day, risk, TODAY } from "./testFixtures";

// jsdom has no layout, so give the chart a fixed size instead of measuring its parent.
vi.mock("recharts", async (importOriginal) => {
	const actual = await importOriginal<typeof import("recharts")>();
	return {
		...actual,
		ResponsiveContainer: ({ children }: { children: ReactElement }) =>
			cloneElement(
				children as ReactElement<{ width: number; height: number }>,
				{ width: 600, height: 220 },
			),
	};
});

const { scale } = risk();

function bars(container: HTMLElement): SVGPathElement[] {
	return Array.from(container.querySelectorAll("path.recharts-rectangle"));
}

// @spec DIS-UI-CHART-003
describe("DailyRiskChart", () => {
	it("draws a tier-colored bar per day, forecast faded and today outlined", () => {
		const daily = risk().daily;
		const { container } = render(
			<DailyRiskChart daily={daily} scale={scale} today={TODAY} />,
		);
		const drawn = bars(container);
		expect(drawn).toHaveLength(daily.length);
		drawn.forEach((bar, i) => {
			const d = daily[i];
			expect(bar.getAttribute("fill")).toBe(RISK_TIER_COLORS[d.tier]);
			expect(bar.getAttribute("fill-opacity")).toBe(
				d.is_forecast ? "0.45" : "1",
			);
			expect(bar.getAttribute("stroke")).toBe(
				d.date === TODAY ? "#2d3748" : null,
			);
		});
	});

	it("clamps bars to the scale and draws dashed tier thresholds", () => {
		const daily = [day("2026-09-19", 14), day(TODAY, -8)];
		const { container } = render(
			<DailyRiskChart daily={daily} scale={scale} today={TODAY} />,
		);
		// Plot area runs y = 10 (scale max) to y = 190 (scale min).
		const [over, under] = bars(container);
		expect(Number(over.getAttribute("y"))).toBeCloseTo(10);
		const underY = Number(under.getAttribute("y"));
		const underEnds = [underY, underY + Number(under.getAttribute("height"))];
		expect(Math.max(...underEnds)).toBeCloseTo(190);

		const lines = Array.from(
			container.querySelectorAll("line.recharts-reference-line-line"),
		);
		expect(lines.map((l) => l.getAttribute("y"))).toEqual(["0", "5", "6"]);
		for (const line of lines) {
			expect(line.getAttribute("stroke-dasharray")).toBe("3 3");
		}
		const labels = Array.from(
			container.querySelectorAll("text.recharts-label"),
		);
		expect(labels.map((l) => l.textContent)).toEqual([
			"Moderate",
			"High",
			"Severe",
		]);
	});

	it("explains fading and outline in the legend", () => {
		render(<DailyRiskChart daily={risk().daily} scale={scale} today={TODAY} />);
		expect(screen.getByText("Faded = forecast")).toBeInTheDocument();
		expect(screen.getByText("Outlined = today")).toBeInTheDocument();
	});

	it("tooltips name the day Partial day, Forecast or Observed", () => {
		const row = (d: ReturnType<typeof day>) => ({
			payload: { ...d, label: "9/21", plotted: d.value },
		});
		const kinds: [ReturnType<typeof day>, string][] = [
			[day("2026-09-19", 4), "Observed"],
			[day("2026-09-21", 6.3), "Forecast"],
			[day("2026-09-22", 3, { partial: true }), "Partial day"],
			[day("2026-09-19", 3, { partial: true }), "Partial day"],
		];
		for (const [d, kind] of kinds) {
			const { unmount } = render(
				<RiskTooltip active payload={[row(d)]} scale={scale} />,
			);
			expect(screen.getByText(`9/21 · ${kind}`)).toBeInTheDocument();
			unmount();
		}
	});

	// @spec DIS-UI-CHART-004
	it("offers the days as a table with score, tier and source", () => {
		render(<DailyRiskChart daily={risk().daily} scale={scale} today={TODAY} />);
		expect(screen.getByText("View as table")).toBeInTheDocument();
		const table = screen.getByRole("table", { hidden: true });
		const rows = within(table)
			.getAllByRole("row", { hidden: true })
			.slice(1)
			.map((r) =>
				within(r)
					.getAllByRole("cell", { hidden: true })
					.map((c) => c.textContent),
			);
		expect(rows).toEqual([
			["9/19", "4.2 E-index", "△ Moderate", "Observed"],
			["Today", "5.4 E-index", "⚠ High", "Observed"],
			["9/21", "6.3 E-index", "! Severe", "Forecast"],
			["9/22", "3.1 E-index", "△ Moderate", "Partial day"],
		]);
	});
});
