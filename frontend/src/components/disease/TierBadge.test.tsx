// @vitest-environment jsdom
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import {
	RISK_TIER_COLORS,
	RISK_TIER_SYMBOLS,
	RISK_TIERS,
} from "../../types/disease";
import TierBadge from "./TierBadge";

// @spec DIS-UI-CONV-001
describe("TierBadge", () => {
	it("pins the status palette and symbols", () => {
		expect(RISK_TIER_COLORS).toEqual({
			Low: "#0ca30c",
			Moderate: "#fab219",
			High: "#ec835a",
			Severe: "#d03b3b",
		});
		expect(RISK_TIER_SYMBOLS).toEqual({
			Low: "✓",
			Moderate: "△",
			High: "⚠",
			Severe: "!",
		});
	});

	it.each(RISK_TIERS)(
		"shows %s as symbol + label in ink on its tint",
		(tier) => {
			render(<TierBadge tier={tier} />);
			const badge = screen.getByText(tier, { exact: false });
			expect(badge).toHaveTextContent(`${RISK_TIER_SYMBOLS[tier]} ${tier}`);
			expect(badge).toHaveStyle({
				borderColor: RISK_TIER_COLORS[tier],
				backgroundColor: `${RISK_TIER_COLORS[tier]}26`,
				color: "#2d3748",
			});
		},
	);
});
