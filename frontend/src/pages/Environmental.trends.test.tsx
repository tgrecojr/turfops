// @vitest-environment jsdom
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
	getEnvironmental,
	getHistorical,
	getSoilTempForecast,
} from "../api/client";
import { ApiError } from "../api/errors";
import {
	EMPTY_HISTORY,
	historical,
	summary,
} from "../test/environmentalFixtures";
import Environmental from "./Environmental";

vi.mock("../api/client", () => ({
	getEnvironmental: vi.fn(),
	getHistorical: vi.fn(),
	getSoilTempForecast: vi.fn(),
	refreshEnvironmental: vi.fn(),
}));

vi.mock("recharts", () => {
	const none = () => null;
	return {
		Area: none,
		AreaChart: none,
		CartesianGrid: none,
		ComposedChart: none,
		Line: none,
		ReferenceLine: none,
		ResponsiveContainer: none,
		Tooltip: none,
		XAxis: none,
		YAxis: none,
	};
});

const CHARTS = [
	"Soil Temperature (10cm)",
	"Ambient Temperature",
	"Humidity",
	"Soil Moisture (10cm)",
	"Precipitation",
	"GDD Accumulation (Base 50°F)",
];

/** What sits under the Historical Trends header and its range selector. */
function trends(): HTMLElement {
	const header = screen.getByText("Historical Trends").parentElement;
	return header?.nextElementSibling as HTMLElement;
}

beforeEach(() => {
	vi.mocked(getEnvironmental).mockResolvedValue(summary());
	// No outlook section, so every "No data available" below is a trend chart.
	vi.mocked(getSoilTempForecast).mockRejectedValue(new Error("503"));
});

afterEach(() => {
	vi.clearAllMocks();
});

describe("Environmental historical trends", () => {
	// @spec WX-VIEW-PAGE-005
	it("loads the selected 7d / 30d / 90d range into six trend charts", async () => {
		vi.mocked(getHistorical).mockImplementation(async (range) =>
			historical(range),
		);
		render(<Environmental />);
		await screen.findByText(CHARTS[0]);
		for (const label of CHARTS) {
			expect(within(trends()).getByText(label)).toBeInTheDocument();
		}
		expect(getHistorical).toHaveBeenLastCalledWith("7d");

		await userEvent.click(screen.getByRole("button", { name: "30d" }));
		expect(getHistorical).toHaveBeenLastCalledWith("30d");
		await userEvent.click(screen.getByRole("button", { name: "90d" }));
		expect(getHistorical).toHaveBeenLastCalledWith("90d");
		expect(await screen.findByText(CHARTS[0])).toBeInTheDocument();
		expect(getHistorical).toHaveBeenCalledTimes(3);
	});

	// @spec WX-VIEW-PAGE-005
	it("says No data available for each empty series", async () => {
		vi.mocked(getHistorical).mockResolvedValue(EMPTY_HISTORY);
		render(<Environmental />);
		await screen.findByText(CHARTS[0]);
		expect(screen.getAllByText("No data available")).toHaveLength(6);
	});

	// @spec WX-VIEW-PAGE-006
	it("clears the charts and shows a plain error when a range fails to load", async () => {
		const lakeDown =
			"The weather lake can't be read right now. Try again; if it persists, check the mount.";
		vi.mocked(getHistorical).mockImplementation(async (range) => {
			if (range === "30d") throw new ApiError(503, lakeDown);
			return historical(range);
		});
		render(<Environmental />);
		expect(await screen.findByText(CHARTS[0])).toBeInTheDocument();

		await userEvent.click(screen.getByRole("button", { name: "30d" }));
		expect(await screen.findByText(lakeDown)).toBeInTheDocument();
		expect(trends()).toHaveTextContent(lakeDown);
		for (const label of CHARTS) {
			expect(within(trends()).queryByText(label)).not.toBeInTheDocument();
		}
	});
});
