// @vitest-environment jsdom
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { SoilTempPrediction, ThresholdPrediction } from "../types";
import SoilTempForecastWidget from "./SoilTempForecastWidget";

function crossing(
	name: string,
	temp: number,
	days: number,
	direction: ThresholdPrediction["direction"] = "Rising",
	date = `2026-10-${String(11 + days).padStart(2, "0")}`,
): ThresholdPrediction {
	return {
		threshold_name: name,
		threshold_temp_f: temp,
		estimated_crossing_date: date,
		days_until_crossing: days,
		beyond_forecast: false,
		confidence: "High",
		direction,
	};
}

function endingAt(temp: number): SoilTempPrediction[] {
	return [
		{
			date: "2026-10-16",
			predicted_soil_temp_f: temp,
			confidence: "Low",
			air_temp_used_f: temp,
			source_description: "test",
		},
	];
}

describe("SoilTempForecastWidget", () => {
	// @spec WX-OUTLOOK-UI-002
	it("headlines the nearest crossing more than 0 days out, then up to two more", () => {
		render(
			<SoilTempForecastWidget
				crossings={[
					crossing("Today Already", 50, 0),
					crossing("Grub Control Window", 60, 3),
					crossing("Active Growth Peak", 65, 4),
					crossing("Heat Stress Risk", 75, 5),
					crossing("Fourth Extra", 80, 6),
				]}
				predictions={endingAt(70)}
				currentSoilTemp={58.24}
			/>,
		);
		expect(screen.getByText("~3d")).toBeInTheDocument();
		expect(screen.getByText("until soil reaches 60°F")).toBeInTheDocument();
		expect(screen.getByText("Grub Control Window")).toBeInTheDocument();
		expect(screen.getByText("Est. Oct 14")).toBeInTheDocument();
		expect(screen.getByText("High")).toBeInTheDocument();
		expect(
			screen.getByText("~4d until soil reaches 65°F (Active Growth Peak)"),
		).toBeInTheDocument();
		expect(
			screen.getByText("~5d until soil reaches 75°F (Heat Stress Risk)"),
		).toBeInTheDocument();
		expect(screen.queryByText(/Fourth Extra/)).not.toBeInTheDocument();
		expect(screen.queryByText(/Today Already/)).not.toBeInTheDocument();
	});

	// @spec WX-OUTLOOK-UI-002
	it("counts a falling crossing only while the last prediction stays below it", () => {
		const falling = crossing("Winterizer / Dormancy", 45, 2, "Falling");
		const { rerender } = render(
			<SoilTempForecastWidget
				crossings={[falling]}
				predictions={endingAt(47)}
				currentSoilTemp={46}
			/>,
		);
		expect(
			screen.getByText("Current: 46.0°F — No threshold crossings predicted"),
		).toBeInTheDocument();

		rerender(
			<SoilTempForecastWidget
				crossings={[falling]}
				predictions={endingAt(43)}
				currentSoilTemp={46}
			/>,
		);
		expect(screen.getByText("~2d")).toBeInTheDocument();
		expect(screen.getByText("until soil drops below 45°F")).toBeInTheDocument();
	});

	// @spec WX-OUTLOOK-UI-002
	it("says no crossings are predicted when none remain", () => {
		render(
			<SoilTempForecastWidget
				crossings={[]}
				predictions={endingAt(62)}
				currentSoilTemp={62.04}
			/>,
		);
		expect(
			screen.getByText("Current: 62.0°F — No threshold crossings predicted"),
		).toBeInTheDocument();
	});
});
