// @vitest-environment jsdom
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { describe, expect, it, vi } from "vitest";
import type { SoilSeriesDay } from "../../types/timing";
import SoilSeasonChart from "./SoilSeasonChart";
import { seriesFor } from "./testFixtures";

// jsdom has no layout, so Recharts draws nothing; stand-ins expose the props instead.
vi.mock("recharts", () => ({
	ResponsiveContainer: ({ children }: { children: ReactNode }) => (
		<div>{children}</div>
	),
	LineChart: ({ data, children }: { data: unknown[]; children: ReactNode }) => (
		<div data-testid="chart" data-rows={JSON.stringify(data)}>
			{children}
		</div>
	),
	Line: (p: { dataKey: string; stroke: string; strokeDasharray?: string }) => (
		<div
			data-testid="line"
			data-key={p.dataKey}
			data-stroke={p.stroke}
			data-dash={p.strokeDasharray ?? ""}
		/>
	),
	YAxis: (p: { ticks: number[] }) => (
		<div data-testid="yaxis" data-ticks={JSON.stringify(p.ticks)} />
	),
	ReferenceLine: (p: { y?: number; x?: string }) =>
		p.y !== undefined ? <div data-testid="threshold" data-y={p.y} /> : null,
	XAxis: () => null,
	CartesianGrid: () => null,
	Tooltip: () => null,
}));

type Row = SoilSeriesDay & { label: string };

function rows(): Row[] {
	return JSON.parse(screen.getByTestId("chart").dataset.rows ?? "[]");
}

function line(key: string): DOMStringMap {
	const el = screen.getAllByTestId("line").find((l) => l.dataset.key === key);
	if (!el) throw new Error(`no ${key} line`);
	return el.dataset;
}

function thresholds(): number[] {
	return screen.getAllByTestId("threshold").map((t) => Number(t.dataset.y));
}

/** Observed through Sep 19 at 66°F, forecast Sep 20–24 at 64°F. */
function fallSeries(): SoilSeriesDay[] {
	return seriesFor(2026).map((d) => ({
		...d,
		this_year: d.date <= "2026-09-19" ? 66 : null,
		forecast: d.date >= "2026-09-20" && d.date <= "2026-09-24" ? 64 : null,
	}));
}

// @spec TIME-UI-006
describe("SoilSeasonChart", () => {
	it("draws this year solid blue, the forecast dashed in the same hue, last year orange, the typical year neutral dotted", () => {
		render(<SoilSeasonChart series={fallSeries()} today="2026-09-20" />);
		expect(line("this_year")).toMatchObject({ stroke: "#2a78d6", dash: "" });
		expect(line("forecast").stroke).toBe("#2a78d6");
		expect(line("forecast").dash).not.toBe("");
		expect(line("last_year")).toMatchObject({ stroke: "#eb6834", dash: "" });
		expect(line("typical").stroke).toBe("#718096");
		expect(line("typical").dash).not.toBe("");
	});

	// @spec UI-CONV-004
	it("explains the Today marker in the legend", () => {
		render(<SoilSeasonChart series={fallSeries()} today="2026-09-20" />);
		const legend = screen
			.getAllByText("Typical year")
			.map((key) => key.parentElement as HTMLElement)
			.find((row) => within(row).queryByText("Last year"));
		expect(legend).toBeDefined();
		expect(
			within(legend as HTMLElement).getByText("Today"),
		).toBeInTheDocument();
	});

	it("joins the forecast to the last observed day", () => {
		render(<SoilSeasonChart series={fallSeries()} today="2026-09-20" />);
		const sep19 = rows().find((r) => r.date === "2026-09-19");
		expect(sep19).toMatchObject({ this_year: 66, forecast: 66 });
		const sep18 = rows().find((r) => r.date === "2026-09-18");
		expect(sep18?.forecast).toBeNull();
	});

	it("uses 10°F ticks spanning the plotted values", () => {
		render(<SoilSeasonChart series={fallSeries()} today="2026-09-20" />);
		const ticks: number[] = JSON.parse(
			screen.getByTestId("yaxis").dataset.ticks ?? "[]",
		);
		expect(ticks).toEqual([50, 60, 70]);
	});

	it("shows the fall thresholds from July and the spring thresholds before", () => {
		const { unmount } = render(
			<SoilSeasonChart series={fallSeries()} today="2026-07-01" />,
		);
		expect(thresholds()).toEqual([75, 70, 65, 55]);
		unmount();
		render(<SoilSeasonChart series={fallSeries()} today="2026-06-30" />);
		expect(thresholds()).toEqual([45, 50, 55, 65]);
	});

	it("shows 75 days before to 60 days after today, with a full-year toggle", async () => {
		render(<SoilSeasonChart series={fallSeries()} today="2026-09-20" />);
		const season = rows();
		expect(season[0].date).toBe("2026-07-07");
		expect(season[season.length - 1].date).toBe("2026-11-19");

		await userEvent.click(
			screen.getByRole("button", { name: "Show full year" }),
		);
		expect(rows()).toHaveLength(365);
		expect(
			screen.getByRole("button", { name: "Show this season" }),
		).toHaveAttribute("aria-pressed", "true");
	});

	it("has a table view of the same days", () => {
		render(<SoilSeasonChart series={fallSeries()} today="2026-09-20" />);
		expect(screen.getByText("View as table")).toBeInTheDocument();
		const table = screen.getByRole("table", { hidden: true });
		expect(table).toHaveTextContent("Sep 19");
		expect(table).toHaveTextContent("66.0°F");
		expect(table).toHaveTextContent("64.0°F (forecast)");
	});
});
