// @vitest-environment jsdom
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { WindowState } from "../../types/timing";
import {
	WINDOW_STATE_COLORS,
	WINDOW_STATE_LABELS,
	WINDOW_STATE_SYMBOLS,
} from "../../types/timing";
import StateBadge from "./StateBadge";

// @spec TIME-UI-002
describe("StateBadge", () => {
	const states = Object.keys(WINDOW_STATE_LABELS) as WindowState[];

	it.each(states)("shows %s as symbol + label on its status tint", (state) => {
		render(<StateBadge state={state} />);
		const badge = screen.getByText(WINDOW_STATE_LABELS[state], {
			exact: false,
		});
		expect(badge).toHaveTextContent(WINDOW_STATE_SYMBOLS[state]);
		expect(badge).toHaveStyle({ borderColor: WINDOW_STATE_COLORS[state] });
		expect(badge).toHaveStyle({ color: "#2d3748" });
	});
});
