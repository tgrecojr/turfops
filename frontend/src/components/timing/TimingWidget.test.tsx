// @vitest-environment jsdom
import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import { describe, expect, it } from "vitest";
import TimingWidget from "./TimingWidget";
import { response } from "./testFixtures";

// @spec TIME-UI-007
describe("TimingWidget", () => {
	it("lists the primary windows by urgency with headline and badge, each linking to /timing", () => {
		render(
			<MemoryRouter>
				<TimingWidget data={response()} />
			</MemoryRouter>,
		);
		const rows = screen
			.getAllByRole("link")
			.filter((a) => a.textContent?.includes("Headline"));
		expect(rows.map((a) => a.textContent)).toEqual([
			expect.stringContaining("Headline fall_seeding"), // Closing
			expect.stringContaining("Headline fall_pre_emergent"), // Ideal
			expect.stringContaining("Headline spring_pre_emergent"), // NotYet
		]);
		for (const row of rows) expect(row).toHaveAttribute("href", "/timing");
		expect(rows[0]).toHaveTextContent("△ Closing");
		expect(rows[1]).toHaveTextContent("★ Ideal now");
		// Secondary and optional windows stay on the Timing page.
		expect(screen.queryByText("Headline spring_seeding")).toBeNull();
		expect(screen.queryByText("Headline dormant_seeding")).toBeNull();
		expect(screen.getByRole("link", { name: /View all/ })).toHaveAttribute(
			"href",
			"/timing",
		);
	});
});
