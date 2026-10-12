// @vitest-environment jsdom
import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import App from "../App";

vi.mock("../api/client", () => ({
	listPlants: vi.fn(async () => []),
	createPlant: vi.fn(),
	deletePlant: vi.fn(),
	refreshPlantPlan: vi.fn(),
	updatePlant: vi.fn(),
}));

// @spec PLANT-UI-001
describe("Landscape route", () => {
	it("serves the Landscape page at /landscape, linked as Landscape", async () => {
		window.history.pushState({}, "", "/landscape");
		render(<App />);
		expect(
			await screen.findByRole("heading", { name: "Landscape" }),
		).toBeInTheDocument();
		expect(screen.getByRole("link", { name: "Landscape" })).toHaveAttribute(
			"href",
			"/landscape",
		);
	});
});
