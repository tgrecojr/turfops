// @vitest-environment jsdom
import { render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { npkAdvice, summary } from "../../test/soilTestFixtures";
import AdvicePanel from "./AdvicePanel";

/** The card under the heading `title`. */
function card(title: string): HTMLElement {
	const heading = screen.getByRole("heading", { name: title });
	const element = heading.parentElement;
	if (!element) throw new Error(`no card for ${title}`);
	return element;
}

describe("AdvicePanel", () => {
	// @spec SOIL-UI-006
	it("shows the pH Adjustment card with both pHs, the amendment, amount and why", () => {
		render(<AdvicePanel summary={summary()} today="2026-10-11" />);
		const ph = within(card("pH Adjustment"));
		expect(ph.getByText("Current pH").nextSibling).toHaveTextContent("6.0");
		expect(ph.getByText("Target pH").nextSibling).toHaveTextContent("6.5");
		expect(ph.getByText("Lime (calcitic or dolomitic)")).toBeInTheDocument();
		expect(ph.getByText("25 lbs/1000 sqft")).toBeInTheDocument();
		expect(ph.getByText(/Apply lime at 25/)).toBeInTheDocument();
	});

	// @spec SOIL-UI-006
	it("shows no pH card when the advice has no pH recommendation", () => {
		render(
			<AdvicePanel
				summary={summary({ ph_recommendation: null })}
				today="2026-10-11"
			/>,
		);
		expect(
			screen.queryByRole("heading", { name: "pH Adjustment" }),
		).not.toBeInTheDocument();
	});

	// @spec SOIL-UI-007
	it("shows the Fertilizer Recommendation card with ratio, amounts, levels, example and budget", () => {
		render(<AdvicePanel summary={summary()} today="2026-10-11" />);
		const fert = within(card("Fertilizer Recommendation"));
		expect(fert.getByText("3-1-2")).toBeInTheDocument();
		expect(fert.getByText("N rate").nextSibling).toHaveTextContent(
			"0.75 lbs/1000sqft",
		);
		const p = fert.getByText("P₂O₅ rate").nextSibling;
		expect(p).toHaveTextContent("0.25 lbs/1000sqft");
		expect(p).toHaveTextContent("Low");
		const k = fert.getByText("K₂O rate").nextSibling;
		expect(k).toHaveTextContent("0.50 lbs/1000sqft");
		expect(k).toHaveTextContent("Low");
		expect(fert.getByText("Example Product").nextSibling).toHaveTextContent(
			"12-4-8 at 6.3 lbs/1000sqft",
		);
		expect(fert.getByText("N Budget Remaining").nextSibling).toHaveTextContent(
			"3.50 lbs/1000sqft",
		);
		expect(fert.getByText(npkAdvice.explanation)).toBeInTheDocument();
	});

	// @spec SOIL-UI-008
	it("lists each deficiency with current and threshold ppm, level and suggestion", () => {
		render(<AdvicePanel summary={summary()} today="2026-10-11" />);
		const table = screen.getByRole("table");
		const headers = within(table)
			.getAllByRole("columnheader")
			.map((h) => h.textContent);
		expect(headers).toEqual([
			"Nutrient",
			"Current (ppm)",
			"Threshold (ppm)",
			"Status",
			"Suggestion",
		]);
		const cells = within(within(table).getAllByRole("row")[1])
			.getAllByRole("cell")
			.map((c) => c.textContent);
		expect(cells[0]).toBe("Calcium");
		expect(cells[1]).toBe("300.0");
		expect(cells[2]).toBe("500.0");
		expect(cells[3]).toContain("Low");
		expect(cells[4]).toBe(
			"Apply gypsum (calcium sulfate) at 20-40 lbs/1000 sqft",
		);
	});

	// @spec SOIL-UI-008
	it("shows no deficiency table when nothing is low", () => {
		render(
			<AdvicePanel
				summary={summary({ micronutrient_recommendations: [] })}
				today="2026-10-11"
			/>,
		);
		expect(screen.queryByRole("table")).not.toBeInTheDocument();
	});
});
