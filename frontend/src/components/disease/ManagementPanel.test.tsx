// @vitest-environment jsdom
import { render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import ManagementPanel from "./ManagementPanel";
import { management, option } from "./testFixtures";

function programTable(title: string): HTMLElement {
	const card = screen.getByRole("heading", { name: title }).parentElement;
	return within(card as HTMLElement).getByRole("table");
}

function classColumn(table: HTMLElement): string[] {
	return within(table)
		.getAllByRole("row")
		.slice(1)
		.map((row) => within(row).getAllByRole("cell")[0].textContent ?? "");
}

describe("ManagementPanel", () => {
	// @spec DIS-UI-DETAIL-003
	it("shows the action, protection, practices, both programs and the notes", () => {
		render(
			<ManagementPanel
				management={management({
					action: "Protected",
					headline: "Protected — Heritage G covers brown patch.",
					protection: {
						product: "Heritage G",
						class_label: "FRAC 11 (Strobilurins)",
						applied_on: "2026-09-18",
						protected_through: "2026-10-09",
						days_remaining: 19,
					},
				})}
			/>,
		);
		expect(screen.getByText("What to do · Protected")).toBeInTheDocument();
		expect(
			screen.getByText("Protected — Heritage G covers brown patch."),
		).toBeInTheDocument();
		expect(
			screen.getByText(
				/Heritage G \(FRAC 11 \(Strobilurins\)\) · applied 9\/18 · covers through about 10\/9 \(19 d left\)/,
			),
		).toBeInTheDocument();
		expect(
			screen.getByText("Water only between 4 and 7 AM."),
		).toBeInTheDocument();
		expect(classColumn(programTable("Preventative program"))).toHaveLength(2);
		expect(
			classColumn(programTable("Curative program — if you see symptoms")),
		).toEqual(["FRAC 3 (DMIs)★ Suggested next"]);
		expect(
			screen.getByText(
				"Rates are product-specific — follow the product label.",
			),
		).toBeInTheDocument();
	});

	it("has no protection line when nothing protects the lawn", () => {
		render(<ManagementPanel management={management()} />);
		expect(screen.queryByText(/covers through about/)).not.toBeInTheDocument();
	});

	// @spec DIS-UI-DETAIL-004
	it("tags the suggested, last-used and on-shelf options", () => {
		render(<ManagementPanel management={management()} />);
		expect(classColumn(programTable("Preventative program"))).toEqual([
			"FRAC 11 (Strobilurins)↻ Used last — rotate",
			"FRAC 7 (SDHI)★ Suggested next✓ On shelf: Xzemplar",
		]);
	});

	// @spec DIS-UI-DETAIL-005
	it("always shows the curative program, with no symptom input", () => {
		const quiet = management({
			action: "NoAction",
			headline: "No action needed — conditions don't favor brown patch.",
			curative: {
				...management().curative,
				options: [option({ frac_class: "Frac1", class_label: "FRAC 1" })],
			},
		});
		const { container } = render(<ManagementPanel management={quiet} />);
		expect(
			screen.getByRole("heading", {
				name: "Curative program — if you see symptoms",
			}),
		).toBeInTheDocument();
		expect(
			classColumn(programTable("Curative program — if you see symptoms")),
		).toEqual(["FRAC 1"]);
		expect(
			container.querySelectorAll("input, select, button, textarea"),
		).toHaveLength(0);
	});
});
