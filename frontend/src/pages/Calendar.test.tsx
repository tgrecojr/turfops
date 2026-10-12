// @vitest-environment jsdom
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { getCalendar, getSeasonalPlan } from "../api/client";
import { ApiError } from "../api/errors";
import { STATUS_META } from "../components/plan/activityStatus";
import type {
	Application,
	ApplicationType,
	CalendarResponse,
	PlannedActivity,
} from "../types";
import { APPLICATION_TYPE_COLORS } from "../types";
import Calendar from "./Calendar";

vi.mock("../api/client", () => ({
	getCalendar: vi.fn(),
	getSeasonalPlan: vi.fn(),
}));
const fetchCalendar = vi.mocked(getCalendar);
const fetchPlan = vi.mocked(getSeasonalPlan);

function app(
	id: number,
	type: ApplicationType,
	date: string,
	extra: Partial<Application> = {},
): Application {
	return {
		id,
		lawn_profile_id: 1,
		application_type: type,
		product_name: null,
		application_date: date,
		rate_per_1000sqft: null,
		coverage_sqft: null,
		notes: null,
		nitrogen_pct: null,
		phosphorus_pct: null,
		potassium_pct: null,
		follow_up_date: null,
		created_at: "2026-06-01T12:00:00Z",
		...extra,
	};
}

function activity(
	id: string,
	start: string,
	end: string,
	extra: Partial<PlannedActivity> = {},
): PlannedActivity {
	return {
		id,
		name: `Name ${id}`,
		category: "Pest Control",
		description: `About ${id}`,
		date_window: {
			predicted_start: start,
			predicted_end: end,
			earliest_historical: null,
			latest_historical: null,
			confidence: "High",
		},
		status: "Active",
		status_estimated: false,
		details: {
			soil_temp_trigger: "60-75°F (7-day avg at 10cm)",
			product_suggestions: ["GrubEx"],
			rate: "Label rate",
			notes: "Water in well.",
		},
		...extra,
	};
}

const seeded = app(1, "Overseed", "2026-05-20", {
	follow_up_date: "2026-06-10",
});
const june: CalendarResponse = {
	year: 2026,
	month: 6,
	days: {
		"2026-05-20": [seeded],
		"2026-06-07": [
			app(2, "Fertilizer", "2026-06-07", {
				product_name: "Milorganite",
				follow_up_date: "2026-07-01",
				notes: "Light feeding",
			}),
		],
	},
};
const grub = activity("grub_preventative", "2026-06-05", "2026-06-20");
const lilac = activity(
	"plant_1_pruning_2026-06-10",
	"2026-06-10",
	"2026-06-12",
	{
		name: "Prune the lilac",
		category: "Plant Maintenance",
		status: "Upcoming",
		description: "Cut back spent blooms.",
	},
);

function plan(activities: PlannedActivity[]) {
	return {
		year: 2026,
		activities,
		data_years_used: 5,
		generated_at: "2026-06-01T12:00:00Z",
		today: "2026-06-01",
	};
}

function cell(day: number): HTMLElement {
	return screen
		.getByText(String(day), { selector: "div" })
		.closest("td") as HTMLElement;
}

/** An outline only: transparent fill, a 2px solid border in `color`. */
function expectRing(el: HTMLElement, color: string) {
	expect(el.style.backgroundColor).toBe("transparent");
	expect(el.style.borderStyle).toBe("solid");
	expect(el.style.borderWidth).toBe("2px");
	expect(el).toHaveStyle({ borderColor: color });
}

beforeEach(() => {
	fetchCalendar.mockReset();
	fetchPlan.mockReset();
	fetchCalendar.mockResolvedValue(june);
	fetchPlan.mockResolvedValue(plan([grub, lilac]));
});

describe("Calendar page", () => {
	// @spec PLAN-CALUI-002
	it("asks for the shown month and that month's year of the plan", async () => {
		render(<Calendar />);
		await screen.findByText("June 2026");
		expect(fetchCalendar).toHaveBeenCalledWith(undefined, undefined);
		expect(fetchPlan).toHaveBeenCalledWith(2026);
		expect(await screen.findAllByRole("img")).not.toHaveLength(0);
	});

	// @spec PLAN-CALUI-002
	it("still shows the applications, without bars or an error, when the plan fails", async () => {
		fetchPlan.mockRejectedValue(new ApiError(500, "boom"));
		render(<Calendar />);
		await screen.findByText("June 2026");
		await vi.waitFor(() => expect(fetchPlan).toHaveBeenCalled());
		expect(within(cell(7)).getByTitle("Fertilizer")).toBeInTheDocument();
		expect(screen.queryAllByRole("img")).toHaveLength(0);
		expect(screen.queryByText(/went wrong|boom/)).not.toBeInTheDocument();
	});

	// @spec PLAN-CALUI-003
	it("marks logged applications with a dot and follow-ups with a ring", async () => {
		render(<Calendar />);
		await screen.findByText("June 2026");
		const dot = within(cell(7)).getByTitle("Fertilizer");
		expect(dot).toHaveStyle({
			backgroundColor: APPLICATION_TYPE_COLORS.Fertilizer,
		});
		const ring = within(cell(10)).getByTitle("Follow-up: Overseed");
		expectRing(ring, APPLICATION_TYPE_COLORS.Overseed);
		expect(within(cell(20)).queryByTitle(/Overseed/)).not.toBeInTheDocument();
	});

	// @spec PLAN-CALUI-004
	it("draws a status bar per activity window on the day and tints the day", async () => {
		render(<Calendar />);
		await screen.findAllByRole("img");
		const day11 = within(cell(11)).getAllByRole("img");
		expect(day11).toHaveLength(2);
		const [turf, plant] = day11;
		expect(turf).toHaveStyle({
			backgroundColor: STATUS_META.Active.color,
			height: "4px",
		});
		expectRing(plant, STATUS_META.Upcoming.color);
		expect(plant).toHaveStyle({ height: "2px" });
		expect(within(cell(4)).queryAllByRole("img")).toHaveLength(0);
		expect(within(cell(21)).queryAllByRole("img")).toHaveLength(0);
		expect(cell(5)).toHaveStyle({ backgroundColor: "#f8faff" });
		expect(cell(4)).toHaveStyle({ backgroundColor: "#fff" });
	});

	// @spec PLAN-CALUI-007
	it("opens a day's details by click and by keyboard", async () => {
		const user = userEvent.setup();
		render(<Calendar />);
		await screen.findAllByRole("img");

		await user.click(cell(7));
		const applications = screen.getByText("Applications", { selector: "h4" })
			.parentElement as HTMLElement;
		expect(within(applications).getByText("Fertilizer")).toBeInTheDocument();
		expect(within(applications).getByText("Milorganite")).toBeInTheDocument();
		expect(
			within(applications).getByText("Follow-up scheduled for 2026-07-01"),
		).toBeInTheDocument();
		expect(within(applications).getByText("Light feeding")).toBeInTheDocument();
		expect(screen.getByText("Planned Activities")).toBeInTheDocument();
		expect(
			screen.getByText("Soil temp: 60-75°F (7-day avg at 10cm)"),
		).toBeInTheDocument();
		expect(screen.getByText("Products: GrubEx")).toBeInTheDocument();
		expect(screen.getByText("Water in well.")).toBeInTheDocument();
		expect(screen.getByText(/High confidence/)).toBeInTheDocument();
		expect(screen.queryByText("Label rate")).not.toBeInTheDocument();

		cell(10).focus();
		await user.keyboard("{Enter}");
		expect(screen.getByText("Follow-ups Due")).toBeInTheDocument();
		expect(
			screen.getByText("Originally applied 2026-05-20"),
		).toBeInTheDocument();
		expect(screen.getByText("Turf Activities")).toBeInTheDocument();
		const plants = screen.getByText("Plant Maintenance")
			.parentElement as HTMLElement;
		expect(within(plants).getByText("○ Upcoming")).toBeInTheDocument();
		expect(within(plants).getByText("Prune the lilac")).toBeInTheDocument();
		expect(within(plants).getByText("Jun 10 – Jun 12")).toBeInTheDocument();
		expect(
			within(plants).getByText("Cut back spent blooms."),
		).toBeInTheDocument();

		cell(5).focus();
		await user.keyboard(" ");
		expect(
			screen.getByRole("heading", { name: "2026-06-05" }),
		).toBeInTheDocument();
	});

	// @spec PLAN-CALUI-008
	it.each([
		[
			new ApiError(404, "No lawn profile found"),
			"There is no lawn profile yet. Set one up in Settings to get started.",
		],
		[
			new ApiError(0, "The operation was aborted", true),
			"That took too long to answer. Try again.",
		],
	])(
		"explains a failed calendar load in plain words (%s)",
		async (err, sentence) => {
			fetchCalendar.mockRejectedValue(err);
			render(<Calendar />);
			const message = await screen.findByText(sentence);
			expect(message.textContent).not.toMatch(/404|\{|aborted/);
		},
	);
});
