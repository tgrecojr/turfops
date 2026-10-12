// @vitest-environment jsdom
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { getRecommendations, patchRecommendation } from "../api/client";
import type { Recommendation } from "../types";
import Recommendations from "./Recommendations";

vi.mock("../api/client", () => ({
	getRecommendations: vi.fn(),
	patchRecommendation: vi.fn(),
}));

const getRecs = vi.mocked(getRecommendations);
const patch = vi.mocked(patchRecommendation);

function rec(id: string, over: Partial<Recommendation> = {}): Recommendation {
	return {
		id,
		category: "GrubControl",
		severity: "Advisory",
		title: `Title ${id}`,
		description: `Description ${id}`,
		explanation: "",
		data_points: [],
		suggested_action: null,
		needs: [],
		created_at: "2026-10-11T12:00:00Z",
		dismissed: false,
		addressed: false,
		...over,
	};
}

const grubs = rec("grubs", {
	severity: "Warning",
	explanation: "Egg-laying is under way.",
	data_points: [{ label: "GDD", value: "640", source: "Calculated" }],
	suggested_action: "Apply a grub preventative.",
	inventory: {
		state: "Partial",
		matches: [
			{
				need: "grub control",
				product_id: 4,
				name: "GrubEx",
				stock_status: "Low",
			},
		],
		missing: ["wetting agent"],
	},
});

async function renderPage(feed: Recommendation[]) {
	getRecs.mockResolvedValue(feed);
	render(
		<MemoryRouter>
			<Recommendations />
		</MemoryRouter>,
	);
	await screen.findByRole("heading", { name: "Recommendations" });
}

function cardTitles() {
	return screen
		.queryAllByText(/^Title /)
		.map((el) => el.textContent)
		.filter((t, i, all) => all.indexOf(t) === i);
}

beforeEach(() => {
	getRecs.mockReset();
	patch.mockReset();
	patch.mockResolvedValue({ id: "", dismissed: false, addressed: false });
});

describe("Recommendations page", () => {
	// @spec FEED-UI-001, FEED-UI-012
	it("fetches the feed once with answered ones and lists the active ones in feed order", async () => {
		await renderPage([
			rec("info", { severity: "Info" }),
			rec("answered", { severity: "Critical", dismissed: true }),
			rec("critical", { severity: "Critical" }),
			rec("advisory"),
		]);
		expect(getRecs).toHaveBeenCalledTimes(1);
		expect(getRecs).toHaveBeenCalledWith(true);
		expect(cardTitles()).toEqual([
			"Title info",
			"Title critical",
			"Title advisory",
		]);
		expect(
			screen.getByRole("button", { name: /Show dismissed \/ addressed \(1\)/ }),
		).toBeInTheDocument();
	});

	// @spec FEED-UI-002
	it("shows each active card's severity, category, inventory, text and answer buttons", async () => {
		await renderPage([grubs]);
		const card = screen.getByRole("button", { name: /Title grubs/ });
		const inCard = within(card);
		expect(inCard.getByText("⚠ Warning")).toBeInTheDocument();
		expect(inCard.getByText("Grub Control")).toBeInTheDocument();
		expect(inCard.getByText("Partly on hand")).toBeInTheDocument();
		expect(inCard.getByText("Description grubs")).toBeInTheDocument();
		expect(
			inCard.getByRole("button", { name: "Mark Addressed" }),
		).toBeInTheDocument();
		expect(inCard.getByRole("button", { name: "Dismiss" })).toBeInTheDocument();
	});

	// @spec FEED-UI-003
	it.each([
		["Mark Addressed", "addressed"],
		["Dismiss", "dismissed"],
	])(
		"%s sends the flag with the current severity and moves the card to answered",
		async (label, flag) => {
			await renderPage([grubs, rec("other")]);
			const card = screen.getByRole("button", { name: /Title grubs/ });
			await userEvent.click(within(card).getByRole("button", { name: label }));
			expect(patch).toHaveBeenCalledWith("grubs", {
				[flag]: true,
				severity: "Warning",
			});
			expect(
				await screen.findByRole("button", {
					name: /Show dismissed \/ addressed \(1\)/,
				}),
			).toBeInTheDocument();
			expect(cardTitles()).toEqual(["Title other"]);
			expect(getRecs).toHaveBeenCalledTimes(1);
		},
	);

	// @spec FEED-UI-004
	it("opens a detail pane with the full advice and the shelf's answer", async () => {
		await renderPage([grubs]);
		await userEvent.click(screen.getByRole("button", { name: /Title grubs/ }));
		const pane = screen.getByRole("heading", { level: 2, name: "Title grubs" })
			.parentElement as HTMLElement;
		const inPane = within(pane);
		expect(inPane.getByText("⚠ Warning")).toBeInTheDocument();
		expect(inPane.getByText("Grub Control")).toBeInTheDocument();
		expect(inPane.getByText("Description grubs")).toBeInTheDocument();
		expect(inPane.getByText("Egg-laying is under way.")).toBeInTheDocument();
		const row = inPane.getByRole("row", { name: /GDD/ });
		expect(row).toHaveTextContent("640");
		expect(row).toHaveTextContent("Calculated");
		expect(inPane.getByText("Apply a grub preventative.")).toBeInTheDocument();
		const product = inPane.getByRole("link", { name: "GrubEx" });
		expect(product).toHaveAttribute("href", "/inventory");
		expect(product.parentElement).toHaveTextContent(
			"GrubEx covers grub control — running low",
		);
		const missing = inPane.getByText(/Nothing on the shelf for wetting agent/);
		expect(within(missing).getByRole("link")).toHaveAttribute(
			"href",
			"/inventory",
		);
	});

	// @spec FEED-UI-005, FEED-UI-006
	it("lists answered advice behind a toggle and restores it", async () => {
		const answered = rec("old", { severity: "Info", addressed: true });
		await renderPage([rec("live"), answered]);
		await userEvent.click(
			screen.getByRole("button", { name: /Show dismissed \/ addressed \(1\)/ }),
		);
		expect(
			screen.getByText(/comes back on its own if it gets more severe/),
		).toHaveTextContent("once the episode it belonged to is over");
		expect(screen.getByText(/Info · Addressed/)).toBeInTheDocument();
		await userEvent.click(screen.getByRole("button", { name: "Restore" }));
		expect(patch).toHaveBeenCalledWith("old", {
			dismissed: false,
			addressed: false,
		});
		expect(cardTitles()).toEqual(["Title live", "Title old"]);
	});

	// @spec FEED-UI-007
	it("says the lawn is looking good when nothing is active", async () => {
		await renderPage([rec("done", { dismissed: true })]);
		expect(
			screen.getByText("No active recommendations. Your lawn is looking good!"),
		).toBeInTheDocument();
	});
});
