// @vitest-environment jsdom
import { render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import App from "../App";

vi.mock("../api/client", async (importOriginal) => ({
	...(await importOriginal<typeof import("../api/client")>()),
	getSoilTests: vi.fn().mockResolvedValue([]),
	getToday: vi.fn().mockResolvedValue({ today: "2026-10-11", timezone: null }),
}));

afterEach(() => {
	window.history.pushState({}, "", "/");
});

describe("Soil Tests route", () => {
	// @spec SOIL-UI-001
	it("serves the page at /soil-tests, listed in navigation as Soil Tests", async () => {
		window.history.pushState({}, "", "/soil-tests");
		render(<App />);
		expect(
			await screen.findByRole("heading", { name: "Soil Tests", level: 1 }),
		).toBeInTheDocument();
		const link = screen.getByRole("link", { name: "Soil Tests" });
		expect(link).toHaveAttribute("href", "/soil-tests");
		expect(link).toHaveAttribute("aria-current", "page");
	});
});
