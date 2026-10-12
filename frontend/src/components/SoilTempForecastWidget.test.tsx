// @vitest-environment jsdom
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { ThresholdPrediction } from "../types";
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

describe("SoilTempForecastWidget", () => {
	// @spec WX-OUTLOOK-UI-002
	it("headlines the nearest crossing, then up to two more", () => {
		render(
			<SoilTempForecastWidget
				crossings={[
					crossing("Grub Control Window", 60, 3),
					crossing("Active Growth Peak", 65, 4),
					crossing("Heat Stress Risk", 75, 5),
					crossing("Fourth Extra", 80, 6),
				]}
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
	});

	// @spec WX-OUTLOOK-UI-002, UI-TYPE-004
	it("shows a falling crossing as the outlook returned it, judging nothing", () => {
		// The outlook only returns a fall that holds (WX-OUTLOOK-CROSS-006); the widget
		// shows it without second-guessing it.
		render(
			<SoilTempForecastWidget
				crossings={[crossing("Winterizer / Dormancy", 45, 2, "Falling")]}
				currentSoilTemp={46}
			/>,
		);
		expect(screen.getByText("~2d")).toBeInTheDocument();
		expect(screen.getByText("until soil drops below 45°F")).toBeInTheDocument();
	});

	// @spec WX-OUTLOOK-UI-002
	it("says no crossings are predicted when none remain", () => {
		render(<SoilTempForecastWidget crossings={[]} currentSoilTemp={62.04} />);
		expect(
			screen.getByText("Current: 62.0°F — No threshold crossings predicted"),
		).toBeInTheDocument();
	});
});
