// @vitest-environment jsdom
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter, Route, Routes } from "react-router";
import { afterEach, describe, expect, it } from "vitest";
import Layout from "./Layout";

const NAV_LABELS = [
	"Dashboard",
	"Applications",
	"Inventory",
	"Landscape",
	"Calendar",
	"Environmental",
	"Recommendations",
	"Disease Risk",
	"Seed & Pre-Em Timing",
	"Soil Tests",
	"Seasonal Plan",
	"Settings",
];

const realMatchMedia = window.matchMedia;

/** Pretend the viewport is (or is not) phone width. */
function viewport(narrow: boolean) {
	window.matchMedia = (query: string) =>
		({
			matches: narrow,
			media: query,
			onchange: null,
			addListener() {},
			removeListener() {},
			addEventListener() {},
			removeEventListener() {},
			dispatchEvent: () => false,
		}) as MediaQueryList;
}

function renderAt(path: string) {
	return render(
		<MemoryRouter initialEntries={[path]}>
			<Routes>
				<Route element={<Layout />}>
					<Route path="*" element={<p>page body</p>} />
				</Route>
			</Routes>
		</MemoryRouter>,
	);
}

afterEach(() => {
	window.matchMedia = realMatchMedia;
});

describe("sidebar", () => {
	// @spec UI-NAV-001
	it("is the Main navigation landmark with the wordmark and twelve links in order", () => {
		viewport(false);
		renderAt("/");
		const nav = screen.getByRole("navigation", { name: "Main navigation" });
		expect(within(nav).getByText("TurfOps")).toBeInTheDocument();
		const links = within(nav).getAllByRole("link");
		expect(links.map((a) => a.textContent)).toEqual(NAV_LABELS);
		expect(screen.getByText("page body")).toBeInTheDocument();
	});

	// @spec UI-NAV-002
	it("marks the active link by weight as well as color, and Dashboard only on /", () => {
		viewport(false);
		renderAt("/timing");
		const nav = screen.getByRole("navigation", { name: "Main navigation" });
		const active = within(nav).getByRole("link", {
			name: "Seed & Pre-Em Timing",
		});
		expect(active).toHaveStyle({
			fontWeight: "700",
			color: "#68d391",
			backgroundColor: "#2d3748",
		});
		const dashboard = within(nav).getByRole("link", { name: "Dashboard" });
		expect(dashboard).toHaveStyle({ fontWeight: "400" });
	});

	// @spec UI-NAV-002
	it("marks Dashboard active on / itself", () => {
		viewport(false);
		renderAt("/");
		const nav = screen.getByRole("navigation", { name: "Main navigation" });
		expect(within(nav).getByRole("link", { name: "Dashboard" })).toHaveStyle({
			fontWeight: "700",
		});
		expect(within(nav).getByRole("link", { name: "Applications" })).toHaveStyle(
			{ fontWeight: "400" },
		);
	});

	// @spec UI-NAV-003
	it("collapses to a menu inside the landmark at phone width, with all twelve links", async () => {
		viewport(true);
		renderAt("/applications");
		const nav = screen.getByRole("navigation", { name: "Main navigation" });
		// No 220 px sidebar: the page is not laid out beside it.
		expect(nav).not.toHaveStyle({ width: "220px" });
		expect(within(nav).queryAllByRole("link")).toHaveLength(0);
		const menu = within(nav).getByRole("button", { name: /menu/i });
		expect(menu).toHaveAttribute("aria-expanded", "false");

		await userEvent.click(menu);
		expect(menu).toHaveAttribute("aria-expanded", "true");
		const links = within(nav).getAllByRole("link");
		expect(links.map((a) => a.textContent)).toEqual(NAV_LABELS);
		expect(within(nav).getByRole("link", { name: "Applications" })).toHaveStyle(
			{ fontWeight: "700" },
		);
		expect(screen.getByText("page body")).toBeInTheDocument();
	});
});
