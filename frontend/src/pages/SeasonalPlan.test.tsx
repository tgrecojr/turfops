// @vitest-environment jsdom
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { getSeasonalPlan } from "../api/client";
import { ApiError } from "../api/errors";
import type { SeasonalPlan as Plan, PlannedActivity } from "../types";
import SeasonalPlan from "./SeasonalPlan";

vi.mock("../api/client", () => ({ getSeasonalPlan: vi.fn() }));
const fetchPlan = vi.mocked(getSeasonalPlan);

function activity(
	id: string,
	overrides: Partial<PlannedActivity> = {},
): PlannedActivity {
	return {
		id,
		name: `Name ${id}`,
		category: "Fertilization",
		description: `About ${id}`,
		date_window: {
			predicted_start: "2026-04-23",
			predicted_end: "2026-05-27",
			earliest_historical: "2026-04-21",
			latest_historical: "2026-05-29",
			confidence: "Medium",
		},
		status: "Upcoming",
		status_estimated: false,
		details: {
			soil_temp_trigger: "55-65°F (7-day avg at 10cm)",
			product_suggestions: ["Milorganite", "30-0-4"],
			rate: "0.5 lb N/1000 sqft",
			notes: "Slow release.",
		},
		...overrides,
	};
}

function plan(activities: PlannedActivity[], overrides: Partial<Plan> = {}) {
	return {
		year: 2026,
		activities,
		data_years_used: 7,
		generated_at: "2026-04-01T12:00:00Z",
		today: "2026-04-01",
		...overrides,
	};
}

const plant = activity("plant_1_pruning_2026-04-01", {
	name: "Prune the lilac",
	category: "Plant Maintenance",
	status: "Active",
});

beforeEach(() => {
	fetchPlan.mockReset();
});

describe("Seasonal Plan page", () => {
	// @spec PLAN-UI-002
	it("says it is analyzing while the plan loads", () => {
		fetchPlan.mockReturnValue(new Promise(() => {}));
		render(<SeasonalPlan />);
		expect(
			screen.getByText(
				"Analyzing historical soil data to build your seasonal plan...",
			),
		).toBeInTheDocument();
	});

	// @spec PLAN-UI-003
	it("names how many years of soil data the plan is built on", async () => {
		fetchPlan.mockResolvedValue(plan([activity("spring_nitrogen")]));
		render(<SeasonalPlan />);
		expect(
			await screen.findByText(
				"Predicted activity windows based on 7 years of soil temperature data",
			),
		).toBeInTheDocument();
	});

	// @spec PLAN-UI-006
	it("lists the cards in the plan's order with everything a card shows", async () => {
		fetchPlan.mockResolvedValue(
			plan([
				activity("spring_nitrogen", { status: "Active" }),
				activity("grub_preventative", {
					category: "Pest Control",
					date_window: {
						...activity("x").date_window,
						predicted_start: "2026-03-01",
						confidence: "High",
					},
				}),
			]),
		);
		render(<SeasonalPlan />);
		const names = await screen.findAllByRole("heading", { level: 3 });
		expect(names.map((h) => h.textContent)).toEqual([
			"Name spring_nitrogen",
			"Name grub_preventative",
		]);
		const card = names[0].parentElement as HTMLElement;
		expect(within(card).getByText("● Active")).toBeInTheDocument();
		expect(within(card).getByText("Fertilization")).toBeInTheDocument();
		expect(within(card).getByText("About spring_nitrogen")).toBeInTheDocument();
		expect(within(card).getByText(/^Apr 23/)).toBeInTheDocument();
		expect(within(card).getByText("56 days remaining")).toBeInTheDocument();
		expect(within(card).getByText("●●○ Medium")).toBeInTheDocument();
		expect(
			within(card).getByRole("button", { name: "Details" }),
		).toBeInTheDocument();
	});

	// @spec PLAN-UI-008
	it("shows only the details an activity has when expanded", async () => {
		const user = userEvent.setup();
		fetchPlan.mockResolvedValue(
			plan([
				activity("spring_nitrogen"),
				activity("core_aeration", {
					details: {
						soil_temp_trigger: null,
						product_suggestions: [],
						rate: null,
						notes: null,
					},
					date_window: {
						...activity("x").date_window,
						earliest_historical: null,
					},
				}),
			]),
		);
		render(<SeasonalPlan />);
		const [full, bare] = await screen.findAllByRole("button", {
			name: "Details",
		});
		await user.click(full);
		expect(screen.getByText("Soil Temp Trigger:")).toBeInTheDocument();
		expect(screen.getByText("55-65°F (7-day avg at 10cm)")).toBeInTheDocument();
		expect(screen.getByText("Rate:")).toBeInTheDocument();
		expect(screen.getByText("0.5 lb N/1000 sqft")).toBeInTheDocument();
		expect(screen.getByText("Milorganite, 30-0-4")).toBeInTheDocument();
		expect(screen.getByText("Slow release.")).toBeInTheDocument();
		expect(screen.getByText("Apr 21 – May 29")).toBeInTheDocument();

		await user.click(screen.getByRole("button", { name: "Less" }));
		await user.click(bare);
		for (const label of [
			"Soil Temp Trigger:",
			"Rate:",
			"Products:",
			"Notes:",
			"Historical range:",
		]) {
			expect(screen.queryByText(label)).not.toBeInTheDocument();
		}
	});

	// @spec PLAN-UI-005
	it("offers a turf / plants filter only when plant tasks are on the plan", async () => {
		const user = userEvent.setup();
		fetchPlan.mockResolvedValue(plan([activity("spring_nitrogen"), plant]));
		render(<SeasonalPlan />);
		await user.click(await screen.findByRole("button", { name: "Plants" }));
		expect(screen.getByText("Prune the lilac")).toBeInTheDocument();
		expect(screen.queryByText("Name spring_nitrogen")).not.toBeInTheDocument();
		expect(screen.getByText("● 1 Active")).toBeInTheDocument();
		expect(screen.queryByText(/Upcoming$/)).not.toBeInTheDocument();

		await user.click(screen.getByRole("button", { name: "Turf" }));
		expect(screen.getByText("Name spring_nitrogen")).toBeInTheDocument();
		expect(screen.getByText("○ 1 Upcoming")).toBeInTheDocument();
		expect(screen.queryByText("● 1 Active")).not.toBeInTheDocument();
	});

	// @spec PLAN-UI-005
	it("has no filter on a plan without plant tasks", async () => {
		fetchPlan.mockResolvedValue(plan([activity("spring_nitrogen")]));
		render(<SeasonalPlan />);
		await screen.findByText("Name spring_nitrogen");
		expect(
			screen.queryByRole("button", { name: "Plants" }),
		).not.toBeInTheDocument();
	});

	// @spec PLAN-UI-009
	it("tells an empty plan apart from an empty filter", async () => {
		const user = userEvent.setup();
		fetchPlan.mockResolvedValue(plan([]));
		const { unmount } = render(<SeasonalPlan />);
		expect(
			await screen.findByText(
				"No activities could be projected for 2026. Insufficient historical data may be the cause.",
			),
		).toBeInTheDocument();
		unmount();

		fetchPlan.mockResolvedValue(plan([plant]));
		render(<SeasonalPlan />);
		await user.click(await screen.findByRole("button", { name: "Turf" }));
		expect(
			screen.getByText("No turf activities for 2026."),
		).toBeInTheDocument();
	});

	// @spec PLAN-UI-010
	it.each([
		[
			new ApiError(404, "No lawn profile found"),
			"There is no lawn profile yet. Set one up in Settings to get started.",
		],
		[
			new ApiError(0, "The operation was aborted", true),
			"That took too long to answer. Try again.",
		],
	])("explains a failed load in plain words (%s)", async (err, sentence) => {
		fetchPlan.mockRejectedValue(err);
		render(<SeasonalPlan />);
		const message = await screen.findByText(new RegExp(sentence));
		expect(message).toHaveTextContent(sentence);
		expect(message.textContent).not.toMatch(/404|\{|aborted/);
	});
});
