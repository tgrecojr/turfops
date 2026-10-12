// @vitest-environment jsdom
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

describe("component test setup", () => {
	// @spec UI-BUILD-010
	it("renders in jsdom with the jest-dom matchers and layout stubs", () => {
		render(<p>rendered</p>);
		expect(screen.getByText("rendered")).toBeInTheDocument();
		expect(typeof window.ResizeObserver).toBe("function");
		expect(() =>
			new window.ResizeObserver(() => {}).observe(document.body),
		).not.toThrow();
		expect(window.matchMedia("(max-width: 760px)").matches).toBe(false);
	});

	// @spec UI-BUILD-010
	it("cleans up what the previous test rendered", () => {
		expect(screen.queryByText("rendered")).not.toBeInTheDocument();
		expect(document.body).toBeEmptyDOMElement();
	});
});
