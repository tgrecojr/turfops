// @vitest-environment jsdom
import { render } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { points, soilForecast } from "../test/environmentalFixtures";
import PredictionChart from "./PredictionChart";

type Props = Record<string, unknown> & { children?: ReactNode };

// Recharts stand-ins that record the props each chart part was given.
const drawn = vi.hoisted(() => [] as { part: string; props: Props }[]);

vi.mock("recharts", () => {
	const part =
		(name: string) =>
		(props: Props): ReactNode => {
			drawn.push({ part: name, props });
			return props.children ?? null;
		};
	return {
		Area: part("Area"),
		CartesianGrid: part("CartesianGrid"),
		ComposedChart: part("ComposedChart"),
		Line: part("Line"),
		ReferenceLine: part("ReferenceLine"),
		ResponsiveContainer: part("ResponsiveContainer"),
		Tooltip: part("Tooltip"),
		XAxis: part("XAxis"),
		YAxis: part("YAxis"),
	};
});

type Row = { time: number; actual: number | null; predicted: number | null };

function parts(name: string): Props[] {
	return drawn.filter((d) => d.part === name).map((d) => d.props);
}

beforeEach(() => {
	drawn.length = 0;
});

describe("PredictionChart", () => {
	// @spec WX-OUTLOOK-UI-001
	it("draws history solid and predictions dashed at local noon, joined by a bridge point", () => {
		const recent = points([60, 61]);
		const { predictions } = soilForecast();
		render(
			<PredictionChart recentSoilTemps={recent} predictions={predictions} />,
		);

		const [area] = parts("Area");
		expect(area.dataKey).toBe("actual");
		expect(area.strokeDasharray).toBeUndefined();
		const [line] = parts("Line");
		expect(line.dataKey).toBe("predicted");
		expect(line.strokeDasharray).toBeTruthy();

		const data = parts("ComposedChart")[0].data as Row[];
		const lastActual = new Date(recent[1].timestamp).getTime();
		expect(data).toContainEqual({
			time: lastActual,
			actual: null,
			predicted: 61,
		});
		for (const p of predictions) {
			expect(data).toContainEqual({
				time: new Date(`${p.date}T12:00:00`).getTime(),
				actual: null,
				predicted: p.predicted_soil_temp_f,
			});
		}
		expect(data.filter((r) => r.actual !== null)).toHaveLength(2);
		expect(data.map((r) => r.time)).toEqual(
			[...data.map((r) => r.time)].sort((a, b) => a - b),
		);
	});

	// @spec UI-CONV-005
	it("draws history and forecast in one hue, solid then dashed", () => {
		const { predictions } = soilForecast();
		const { container } = render(
			<PredictionChart
				recentSoilTemps={points([60, 61])}
				predictions={predictions}
			/>,
		);
		const [area] = parts("Area");
		const [line] = parts("Line");
		expect(area.stroke).toBe("#2a78d6");
		expect(line.stroke).toBe("#2a78d6");
		expect(line.strokeDasharray).toBeTruthy();
		const html = container.innerHTML.toLowerCase();
		for (const old of ["#e67e22", "#3b82f6"]) {
			expect(html).not.toContain(old);
		}
	});

	// @spec WX-OUTLOOK-UI-001
	it("draws reference lines only at 60°F Grub Control and 75°F Heat Stress", () => {
		render(
			<PredictionChart
				recentSoilTemps={points([60])}
				predictions={soilForecast().predictions}
			/>,
		);
		const lines = parts("ReferenceLine").map((p) => ({
			y: p.y,
			label: (p.label as { value: string }).value,
		}));
		expect(lines).toEqual([
			{ y: 60, label: "60° Grub Control" },
			{ y: 75, label: "75° Heat Stress" },
		]);
	});
});
