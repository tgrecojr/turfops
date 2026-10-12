// @vitest-environment jsdom
import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { getTimingWindows } from "../api/client";
import { ApiError } from "../api/errors";
import { response, stat } from "../components/timing/testFixtures";
import Timing from "./Timing";

vi.mock("../api/client", () => ({ getTimingWindows: vi.fn() }));
// The chart has its own tests; here it only marks its place on the page.
vi.mock("../components/timing/SoilSeasonChart", () => ({
	default: () => <div>soil-season-chart</div>,
}));

const mocked = vi.mocked(getTimingWindows);

/** Position of the first node containing `text` in document order. */
function positionOf(text: string): number {
	const all = document.body.textContent ?? "";
	const at = all.indexOf(text);
	expect(at, `"${text}" on the page`).toBeGreaterThanOrEqual(0);
	return at;
}

function rejectWith(err: ApiError) {
	mocked.mockImplementation(() => Promise.reject(err));
}

// @spec TIME-UI-001
describe("Timing page layout", () => {
	it("shows notes, tiles, context, primary cards by urgency, chart, other options, then the method", async () => {
		mocked.mockResolvedValue(response());
		render(<Timing />);
		await screen.findByText("Headline fall_seeding");

		const order = [
			"Forecast unavailable — no soil outlook",
			"5 cm soil · 5-day mean",
			"Last spring freeze",
			"First fall freeze",
			"running 1.5°F warmer",
			"Freeze in the forecast",
			"Headline fall_seeding", // Closing
			"Headline fall_pre_emergent", // Ideal
			"Headline spring_pre_emergent", // NotYet
			"soil-season-chart",
			"Other seeding options",
			"Headline spring_seeding",
			"Headline dormant_seeding",
			"How this is calculated",
		].map(positionOf);
		expect(order).toEqual([...order].sort((a, b) => a - b));
	});

	it("fills the tiles: soil mean with as-of date, spring freeze as late as p90, fall freeze as early as p10", async () => {
		mocked.mockResolvedValue(response());
		render(<Timing />);
		await screen.findByText("66.2°F");
		expect(screen.getByText("as of Sep 19")).toBeInTheDocument();
		expect(screen.getByText("Apr 10")).toBeInTheDocument();
		expect(screen.getByText("as late as Apr 25 · 12 yr")).toBeInTheDocument();
		expect(screen.getByText("Nov 5")).toBeInTheDocument();
		expect(screen.getByText("as early as Oct 22 · 12 yr")).toBeInTheDocument();
	});

	it("shows this year's fall freeze once it has been seen", async () => {
		const base = response();
		mocked.mockResolvedValue(
			response({
				freeze: {
					...base.freeze,
					first_fall: stat("2026-11-05", { p10: "2026-10-22" }),
					first_fall_this_year: "2026-10-18",
				},
			}),
		);
		render(<Timing />);
		await screen.findByText("this year: Oct 18");
		expect(screen.queryByText(/as early as/)).not.toBeInTheDocument();
	});
});

// @spec TIME-UI-008
describe("Timing page errors", () => {
	it("tells an owner with no lawn profile to set one up in Settings", async () => {
		rejectWith(new ApiError(404, "No lawn profile found"));
		render(<Timing />);
		const msg = await screen.findByText(/Set one up in Settings/);
		expect(msg.textContent).not.toMatch(/404/);
	});

	it("says the weather lake is unavailable, in the server's sentence, without a status code or JSON", async () => {
		const sentence =
			"The weather data lake can't be read right now, so soil, disease, timing and history data are unavailable. Try again in a few minutes; if it persists, check that the lake is mounted at DATALAKE_ROOT.";
		rejectWith(new ApiError(503, sentence));
		render(<Timing />);
		const msg = await screen.findByText(sentence);
		expect(msg.textContent).not.toMatch(/503|\{/);
	});

	it("never shows internal detail for a server failure", async () => {
		rejectWith(new ApiError(500, '{"error":"duckdb: IO Error /data/silver"}'));
		render(<Timing />);
		const msg = await screen.findByText(/Something went wrong on the server/);
		expect(msg.textContent).not.toMatch(/duckdb|500|\{/);
	});
});
