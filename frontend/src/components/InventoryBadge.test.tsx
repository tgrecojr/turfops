// @vitest-environment jsdom
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { Recommendation } from "../types";
import type { InventoryState, InventoryStatus } from "../types/inventory";
import AlertCard from "./AlertCard";
import InventoryBadge from "./InventoryBadge";

const status = (state: InventoryState): InventoryStatus => ({
	state,
	matches: [
		{
			need: "grass seed",
			product_id: 1,
			name: "TTTF blend",
			stock_status: "InStock",
		},
		{
			need: "starter fertilizer",
			product_id: 2,
			name: "Starter",
			stock_status: "Low",
		},
	],
	missing: ["lime"],
});

// @spec INV-UI-009
describe("InventoryBadge", () => {
	it.each([
		["OnHand", "✓", "On hand", "#0ca30c"],
		["Low", "△", "On hand, running low", "#fab219"],
		["NotOnHand", "✕", "Need to buy", "#d03b3b"],
		["Partial", "◐", "Partly on hand", "#fab219"],
	] as [InventoryState, string, string, string][])(
		"shows %s as symbol, label and status color",
		(state, symbol, label, color) => {
			render(<InventoryBadge status={status(state)} />);
			const badge = screen.getByText(label, { exact: false });
			expect(badge).toHaveTextContent(`${symbol} ${label}`);
			expect(badge).toHaveStyle({ borderColor: color });
		},
	);

	it("lists each need's product, its stock when not in stock, or not on shelf", () => {
		render(<InventoryBadge status={status("Partial")} />);
		expect(
			screen.getByText("Partly on hand", { exact: false }),
		).toHaveAttribute(
			"title",
			"grass seed: TTTF blend\nstarter fertilizer: Starter (running low)\nlime: not on shelf",
		);
	});

	it("appears on a dashboard alert card that carries a status", () => {
		const rec = {
			id: "timing_fall_seeding_2026",
			category: "Seeding",
			severity: "Advisory",
			title: "Fall seeding window open",
			description: "Seed now.",
			explanation: "",
			data_points: [],
			suggested_action: null,
			created_at: "2026-09-01T00:00:00Z",
			dismissed: false,
			addressed: false,
			inventory: status("NotOnHand"),
		} as unknown as Recommendation;
		render(<AlertCard rec={rec} />);
		expect(screen.getByText("Need to buy", { exact: false })).toBeVisible();
	});
});
