// @vitest-environment jsdom
import { act, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";

// Every page is replaced by a stub naming itself, so the shell is tested on its own.
const gate = vi.hoisted(() => {
	let open: () => void = () => {};
	const opened = new Promise<void>((resolve) => {
		open = resolve;
	});
	return { opened, open: () => open(), failing: { armed: false } };
});

function stub(name: string) {
	return { default: () => <p>{name} page</p> };
}

vi.mock("./pages/Dashboard", () => stub("Dashboard"));
vi.mock("./pages/Applications", () => stub("Applications"));
vi.mock("./pages/Landscape", () => stub("Landscape"));
vi.mock("./pages/Environmental", () => stub("Environmental"));
vi.mock("./pages/Recommendations", () => stub("Recommendations"));
vi.mock("./pages/DiseaseRisk", () => stub("DiseaseRisk"));
vi.mock("./pages/Timing", () => stub("Timing"));
vi.mock("./pages/SoilTests", () => stub("SoilTests"));
vi.mock("./pages/SeasonalPlan", () => stub("SeasonalPlan"));
// A page whose chunk takes a while to arrive.
vi.mock("./pages/Calendar", async () => {
	await gate.opened;
	return stub("Calendar");
});
// A page whose chunk fails to load.
vi.mock("./pages/Settings", () => {
	throw new Error("Failed to fetch dynamically imported module");
});
// A page that throws while rendering, until the test lets it render.
vi.mock("./pages/Inventory", () => ({
	default: () => {
		if (gate.failing.armed) {
			throw new Error("TypeError at /app/src/pages/Inventory.tsx:12 — secret");
		}
		return <p>Inventory page</p>;
	},
}));

function renderAt(path: string) {
	window.history.pushState({}, "", path);
	return render(<App />);
}

/** jsdom's own console, where it reports what it cannot do (such as a page load). */
function jsdomConsole(): {
	on: (event: "jsdomError", listener: (e: Error) => void) => void;
} {
	return (globalThis as unknown as { jsdom: { virtualConsole: never } }).jsdom
		.virtualConsole;
}

let consoleError: ReturnType<typeof vi.spyOn>;
beforeEach(() => {
	// React reports the render errors the boundary catches on the console.
	consoleError = vi.spyOn(console, "error").mockImplementation(() => {});
});
afterEach(() => {
	consoleError.mockRestore();
});

describe("routes", () => {
	// @spec UI-SHELL-001
	it.each([
		["/", "Dashboard"],
		["/applications", "Applications"],
		["/inventory", "Inventory"],
		["/landscape", "Landscape"],
		["/environmental", "Environmental"],
		["/recommendations", "Recommendations"],
		["/disease-risk", "DiseaseRisk"],
		["/disease-risk/brown-patch", "DiseaseRisk"],
		["/timing", "Timing"],
		["/soil-tests", "SoilTests"],
		["/seasonal-plan", "SeasonalPlan"],
	])("renders %s inside the layout", async (path, page) => {
		renderAt(path);
		expect(await screen.findByText(`${page} page`)).toBeInTheDocument();
		expect(
			screen.getByRole("navigation", { name: "Main navigation" }),
		).toBeInTheDocument();
	});

	// @spec UI-SHELL-002
	it("shows Page Not Found inside the layout, linking back to the dashboard", async () => {
		renderAt("/no/such/page");
		expect(await screen.findByText("Page Not Found")).toBeInTheDocument();
		expect(
			screen.getByRole("navigation", { name: "Main navigation" }),
		).toBeInTheDocument();
		const link = screen.getByRole("link", { name: "Go to Dashboard" });
		expect(link).toHaveAttribute("href", "/");
	});
});

describe("page chunks", () => {
	// @spec UI-SHELL-003, UI-SHELL-001
	it("show Loading... while a page chunk loads", async () => {
		renderAt("/calendar");
		expect(await screen.findByText("Loading...")).toBeInTheDocument();
		await act(async () => gate.open());
		expect(await screen.findByText("Calendar page")).toBeInTheDocument();
		expect(screen.queryByText("Loading...")).not.toBeInTheDocument();
	});
});

describe("error boundary", () => {
	// @spec UI-SHELL-004, UI-ERR-011
	it("replaces the app when a page chunk fails to load", async () => {
		renderAt("/settings");
		expect(await screen.findByText("Something went wrong")).toBeInTheDocument();
		expect(
			screen.queryByRole("navigation", { name: "Main navigation" }),
		).not.toBeInTheDocument();
		expect(screen.queryByText(/dynamically imported/)).not.toBeInTheDocument();
		expect(
			screen.getByRole("button", { name: "Return to Dashboard" }),
		).toBeInTheDocument();
	});

	// @spec UI-SHELL-004, UI-ERR-011
	it("hides the exception text and returns to the dashboard with the error cleared", async () => {
		gate.failing.armed = true;
		renderAt("/inventory");
		const heading = await screen.findByText("Something went wrong");
		const panel = heading.parentElement as HTMLElement;
		expect(panel.textContent).not.toMatch(/secret|TypeError|Inventory\.tsx/);
		expect(within(panel).getAllByRole("button")).toHaveLength(1);

		const jsdomErrors: string[] = [];
		jsdomConsole().on("jsdomError", (e) => jsdomErrors.push(e.message));
		gate.failing.armed = false;
		await userEvent.click(
			screen.getByRole("button", { name: "Return to Dashboard" }),
		);
		// The boundary cleared its error (the app renders again) and asked the browser
		// for a full page load (jsdom reports that it cannot load another document).
		expect(screen.queryByText("Something went wrong")).not.toBeInTheDocument();
		expect(jsdomErrors).toEqual([
			expect.stringMatching(/navigation to another Document/),
		]);
	});
});
