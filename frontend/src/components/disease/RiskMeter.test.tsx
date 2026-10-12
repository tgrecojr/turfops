// @vitest-environment jsdom
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { RiskTier } from "../../types/disease";
import { RISK_TIER_COLORS, RISK_TIERS } from "../../types/disease";
import RiskMeter from "./RiskMeter";
import { risk } from "./testFixtures";

const { scale } = risk();

function bands(container: HTMLElement): HTMLElement[] {
	const track = container.querySelector('[role="img"] > div');
	return Array.from(track?.children ?? []) as HTMLElement[];
}

function fullStrength(container: HTMLElement): RiskTier[] {
	return bands(container)
		.map((band, i) => [band, RISK_TIERS[i]] as const)
		.filter(([band]) => band.style.opacity === "1")
		.map(([, tier]) => tier);
}

// @spec DIS-UI-CHART-001
describe("RiskMeter", () => {
	it("lays the four bands on the native scale with the score's band at full strength", () => {
		const { container } = render(
			<RiskMeter scale={scale} score={5.4} tier="High" />,
		);
		const laid = bands(container);
		expect(laid).toHaveLength(4);
		// E-index −5…10: Low spans 5, Moderate 5, High 1, Severe 4 of 15.
		const grow = laid.map((b) => Number(b.style.flexGrow).toFixed(4));
		expect(grow).toEqual(
			[5 / 15, 5 / 15, 1 / 15, 4 / 15].map((g) => g.toFixed(4)),
		);
		laid.forEach((band, i) => {
			expect(band).toHaveStyle({
				backgroundColor: RISK_TIER_COLORS[RISK_TIERS[i]],
			});
		});
		expect(fullStrength(container)).toEqual(["High"]);
		expect(screen.getByRole("img")).toHaveAccessibleName("Risk meter: High");
	});

	it("puts the marker at the score and ticks the thresholds beneath", () => {
		const { container } = render(
			<RiskMeter scale={scale} score={2.5} tier="Moderate" />,
		);
		const marker = container.querySelector('[role="img"] > div:nth-child(2)');
		expect((marker as HTMLElement).style.left).toBe("50%");
		const ticks = Array.from(
			container.querySelectorAll('[role="img"] > div:nth-child(3) > span'),
		).map((t) => [t.textContent, (t as HTMLElement).style.left]);
		expect(ticks).toEqual([
			["0", `${(5 / 15) * 100}%`],
			["5", `${(10 / 15) * 100}%`],
			["6", `${(11 / 15) * 100}%`],
		]);
	});

	it("clamps an out-of-range score to the ends and hides ticks when compact", () => {
		const { container } = render(
			<RiskMeter scale={scale} score={14} tier="Severe" compact />,
		);
		const marker = container.querySelector('[role="img"] > div:nth-child(2)');
		expect((marker as HTMLElement).style.left).toBe("100%");
		expect(fullStrength(container)).toEqual(["Severe"]);
		expect(container.querySelectorAll("span")).toHaveLength(0);
	});

	// @spec DIS-UI-CHART-002
	it("stays on the weather scale when history raised the tier", () => {
		// Gray leaf spot raised from Moderate to High: the badge says High, the meter's
		// highlighted band is still the score's own (Moderate).
		const { container } = render(
			<RiskMeter
				scale={{
					...scale,
					min: 0,
					max: 100,
					moderate_at: 25,
					high_at: 50,
					severe_at: 75,
				}}
				score={30}
				tier="High"
			/>,
		);
		expect(fullStrength(container)).toEqual(["Moderate"]);
	});
});
