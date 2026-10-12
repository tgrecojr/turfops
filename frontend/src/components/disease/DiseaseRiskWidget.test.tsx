// @vitest-environment jsdom
import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import { describe, expect, it } from "vitest";
import DiseaseRiskWidget from "./DiseaseRiskWidget";
import { response } from "./testFixtures";

// @spec DIS-UI-WIDGET-001
describe("DiseaseRiskWidget", () => {
	it("lists every disease with score label and tier, linking to its detail", () => {
		render(
			<MemoryRouter>
				<DiseaseRiskWidget data={response()} />
			</MemoryRouter>,
		);
		const brownPatch = screen.getByRole("link", { name: /Brown Patch/ });
		expect(brownPatch).toHaveAttribute("href", "/disease-risk/brown-patch");
		expect(brownPatch).toHaveTextContent("Brown PatchE-index 5.4⚠ High");
		const dollarSpot = screen.getByRole("link", { name: /Dollar Spot/ });
		expect(dollarSpot).toHaveAttribute("href", "/disease-risk/dollar-spot");
		expect(dollarSpot).toHaveTextContent("Dollar Spot4% probability✓ Low");
		expect(screen.getByRole("link", { name: "View all ›" })).toHaveAttribute(
			"href",
			"/disease-risk",
		);
	});

	it("shows the first data note when no disease is scored", () => {
		render(
			<MemoryRouter>
				<DiseaseRiskWidget
					data={response({
						diseases: [],
						data_notes: ["No recent station observations are available.", "x"],
					})}
				/>
			</MemoryRouter>,
		);
		expect(
			screen.getByText("No recent station observations are available."),
		).toBeInTheDocument();
		expect(screen.queryByText("x")).not.toBeInTheDocument();
	});
});
