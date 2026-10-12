// @vitest-environment jsdom
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { boundary, stat, timingWindow } from "./testFixtures";
import WindowTimeline from "./WindowTimeline";

/** The colored bars inside the track, by background color. */
function bars(container: HTMLElement): string[] {
	return Array.from(container.querySelectorAll<HTMLElement>("div"))
		.map((el) => el.style.backgroundColor)
		.filter((c) => c !== "");
}

// jsdom reports inline colors as rgb().
const SPREAD = "rgb(213, 228, 247)"; // #d5e4f7
const TYPICAL = "rgb(141, 181, 232)"; // #8db5e8
const IDEAL = "rgb(42, 120, 214)"; // #2a78d6

// @spec TIME-UI-005
describe("WindowTimeline", () => {
	it("draws one blue ramp — record spread, typical window, ideal stretch — with a Today marker and an aria-label", () => {
		const w = timingWindow("fall_pre_emergent", "Ideal", {
			ideal_until: boundary({ typical: stat("2026-09-23") }),
		});
		const { container } = render(
			<WindowTimeline window={w} today="2026-09-20" />,
		);
		expect(
			screen.getByRole("img", {
				name: "Typically opens Sep 7 and closes Oct 20",
			}),
		).toBeInTheDocument();
		const colors = bars(container);
		expect(colors).toContain(SPREAD);
		expect(colors).toContain(TYPICAL);
		expect(colors).toContain(IDEAL);
		expect(screen.getByText("Today")).toBeInTheDocument();
		expect(screen.getByText("Earliest–latest on record")).toBeInTheDocument();
		expect(screen.getByText("Ideal stretch")).toBeInTheDocument();
	});

	it("has no ideal stretch for a window without one", () => {
		const w = timingWindow("spring_seeding", "Open");
		const { container } = render(
			<WindowTimeline window={w} today="2026-09-20" />,
		);
		expect(bars(container)).not.toContain(IDEAL);
		expect(screen.queryByText("Ideal stretch")).toBeNull();
	});

	it("is hidden when the open or close has no typical date", () => {
		const noOpen = timingWindow("fall_seeding", "NotYet", {
			opens: boundary({ typical: null }),
		});
		const { container: a } = render(
			<WindowTimeline window={noOpen} today="2026-09-20" />,
		);
		expect(a).toBeEmptyDOMElement();

		const noClose = timingWindow("fall_seeding", "NotYet", {
			closes: boundary({ typical: null }),
		});
		const { container: b } = render(
			<WindowTimeline window={noClose} today="2026-09-20" />,
		);
		expect(b).toBeEmptyDOMElement();
	});
});
