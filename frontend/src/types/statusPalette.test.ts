import { describe, expect, it } from "vitest";
import { FACTOR_STATUS_COLORS, RISK_TIER_COLORS } from "./disease";
import { INVENTORY_STATE_COLORS, STOCK_STATUS_COLORS } from "./inventory";
import { WINDOW_STATE_COLORS } from "./timing";

const GOOD = "#0ca30c";
const WARNING = "#fab219";
const SERIOUS = "#ec835a";
const CRITICAL = "#d03b3b";
const INFORMATIONAL = "#3182ce";
const NEUTRAL = ["#a0aec0", "#718096"];

describe("status badge colors", () => {
	// @spec UI-CONV-002
	it("come from the status palette, one role per meaning", () => {
		expect(RISK_TIER_COLORS).toEqual({
			Low: GOOD,
			Moderate: WARNING,
			High: SERIOUS,
			Severe: CRITICAL,
		});
		expect(FACTOR_STATUS_COLORS).toEqual({
			Unfavorable: GOOD,
			Marginal: WARNING,
			Favorable: CRITICAL,
		});
		expect(STOCK_STATUS_COLORS).toEqual({
			InStock: GOOD,
			Low: WARNING,
			Out: CRITICAL,
		});
		expect(INVENTORY_STATE_COLORS).toEqual({
			OnHand: GOOD,
			Low: WARNING,
			Partial: WARNING,
			NotOnHand: CRITICAL, // "Need to buy"
		});
		const { NotYet, Closed, Blocked, ...rest } = WINDOW_STATE_COLORS;
		expect(rest).toEqual({
			Open: GOOD,
			Ideal: GOOD,
			Closing: WARNING,
			OpeningSoon: INFORMATIONAL,
			Done: INFORMATIONAL,
		});
		for (const neutral of [NotYet, Closed, Blocked]) {
			expect(NEUTRAL).toContain(neutral);
		}
	});
});
