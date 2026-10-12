// @vitest-environment jsdom
import { render, screen } from "@testing-library/react";
import type { ReactNode } from "react";
import { describe, expect, it, vi } from "vitest";
import type { CrabgrassStatus, GddSummary } from "../types";
import { CRABGRASS_STATUS_COLORS, CRABGRASS_STATUS_LABELS } from "../types";
import GddWidget from "./GddWidget";

// jsdom has no layout, so Recharts draws nothing: stand in for the pieces the widget uses
// and record what it hands them.
vi.mock("recharts", () => ({
	ResponsiveContainer: ({ children }: { children: ReactNode }) => (
		<div>{children}</div>
	),
	AreaChart: ({
		data,
		children,
	}: {
		data: { gdd: number }[];
		children: ReactNode;
	}) => (
		<svg
			role="img"
			aria-label="GDD sparkline"
			data-testid="sparkline"
			data-points={data.length}
			data-first={data[0]?.gdd}
			data-last={data[data.length - 1]?.gdd}
		>
			{children}
		</svg>
	),
	ReferenceLine: ({
		y,
		strokeDasharray,
	}: {
		y: number;
		strokeDasharray?: string;
	}) => <line data-testid="reference" data-y={y} data-dash={strokeDasharray} />,
	Area: () => null,
	YAxis: () => null,
}));

function summary(
	total: number,
	status: CrabgrassStatus,
	days: number,
): GddSummary {
	const daily_history = Array.from({ length: days }, (_, i) => ({
		date: `2026-${String(Math.floor(i / 28) + 1).padStart(2, "0")}-${String((i % 28) + 1).padStart(2, "0")}`,
		high_temp_f: 70,
		low_temp_f: 50,
		gdd_base50: 1,
		cumulative_gdd_base50: i + 1,
	}));
	return {
		year: 2026,
		current_gdd_total: total,
		daily_history,
		crabgrass_model: {
			germination_threshold: 200,
			current_gdd: total,
			status,
			estimated_germination_date: null,
		},
		last_computed_date: daily_history.at(-1)?.date ?? null,
	};
}

// @spec RULE-UI-001
describe("GddWidget", () => {
	it("shows the total against 200 with a bar, a status badge and the footnote", () => {
		const { container } = render(
			<GddWidget data={summary(137, "ApproachingGermination", 3)} />,
		);
		expect(screen.getByText("137")).toBeInTheDocument();
		expect(screen.getByText("/ 200", { exact: false })).toBeInTheDocument();
		expect(
			container.querySelector('div[style*="width: 68.5%"]'),
		).not.toBeNull();
		const badge = screen.getByText(
			CRABGRASS_STATUS_LABELS.ApproachingGermination,
		);
		expect(badge).toHaveStyle({
			color: CRABGRASS_STATUS_COLORS.ApproachingGermination,
		});
		expect(
			screen.getByText("Crabgrass germinates at ~200 GDD"),
		).toBeInTheDocument();
	});

	it("caps the bar at full past the threshold", () => {
		const { container } = render(
			<GddWidget data={summary(450, "PostGermination", 3)} />,
		);
		expect(container.querySelector('div[style*="width: 100%"]')).not.toBeNull();
		expect(
			screen.getByText(CRABGRASS_STATUS_LABELS.PostGermination),
		).toBeInTheDocument();
	});

	it("draws the last 60 days' running total with a dashed line at 200", () => {
		render(<GddWidget data={summary(90, "ApproachingGermination", 90)} />);
		const spark = screen.getByTestId("sparkline");
		expect(spark).toHaveAttribute("data-points", "60");
		expect(spark).toHaveAttribute("data-first", "31");
		expect(spark).toHaveAttribute("data-last", "90");
		const line = screen.getByTestId("reference");
		expect(line).toHaveAttribute("data-y", "200");
		expect(line.getAttribute("data-dash")).toBeTruthy();
	});

	it("draws no sparkline with five points or fewer", () => {
		render(<GddWidget data={summary(5, "PreGermination", 5)} />);
		expect(screen.queryByTestId("sparkline")).toBeNull();
		expect(screen.queryByTestId("reference")).toBeNull();
		render(<GddWidget data={summary(6, "PreGermination", 6)} />);
		expect(screen.getByTestId("sparkline")).toHaveAttribute("data-points", "6");
	});
});
