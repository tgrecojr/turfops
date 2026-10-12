// @vitest-environment jsdom
import { render, screen, within } from "@testing-library/react";
import { cloneElement, type ReactElement } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { FACTOR_STATUS_COLORS, RISK_TIER_COLORS } from "../../types/disease";
import DiseaseDetail from "./DiseaseDetail";
import { management, risk, TODAY } from "./testFixtures";

vi.mock("recharts", async (importOriginal) => {
	const actual = await importOriginal<typeof import("recharts")>();
	return {
		...actual,
		ResponsiveContainer: ({ children }: { children: ReactElement }) =>
			cloneElement(
				children as ReactElement<{ width: number; height: number }>,
				{ width: 600, height: 220 },
			),
	};
});

afterEach(() => {
	vi.useRealTimers();
});

function before(a: Element, b: Element): boolean {
	return Boolean(
		a.compareDocumentPosition(b) & Node.DOCUMENT_POSITION_FOLLOWING,
	);
}

describe("DiseaseDetail", () => {
	// @spec DIS-UI-DETAIL-001
	it("shows score card, chart, summary, management, factors and method in order", () => {
		render(<DiseaseDetail risk={risk()} today={TODAY} />);
		const parts = [
			screen.getByText("E-index 5.4"),
			screen.getByText("Current risk · today"),
			screen.getByRole("img", { name: "Risk meter: High" }),
			screen.getByRole("heading", {
				name: "Daily risk — past week and outlook",
			}),
			screen.getByText("The brown patch E-index is 5.4."),
			screen.getByText(/What to do ·/),
			screen.getByRole("heading", { name: "Contributing factors · today" }),
			screen.getByText("How this is calculated"),
		];
		for (let i = 1; i < parts.length; i++) {
			expect(before(parts[i - 1], parts[i])).toBe(true);
		}
		const badge = screen.getByText("E-index 5.4").nextElementSibling;
		expect(badge).toHaveTextContent("⚠ High");
		expect((badge as HTMLElement).style.fontSize).toBe("0.9rem");
		expect(screen.getByText("The brown patch E-index is 5.4.")).toHaveStyle({
			borderLeftColor: RISK_TIER_COLORS.High,
		});
		const method = screen
			.getByText("How this is calculated")
			.closest("details");
		expect(method).not.toBeNull();
		const inMethod = within(method as HTMLElement);
		expect(
			inMethod.getByText("A field-validated brown patch warning model."),
		).toBeInTheDocument();
		expect(inMethod.getAllByRole("listitem")).toHaveLength(2);
		expect(inMethod.getByText("Rhizoctonia solani")).toBeInTheDocument();
		expect(inMethod.getByText(/Fidanza et al\. \(1996\)/)).toBeInTheDocument();
		expect(inMethod.queryByText(/Experimental/)).not.toBeInTheDocument();
	});

	// @spec DIS-UI-DETAIL-002
	it("opens the method with an Experimental banner for an unvalidated model", () => {
		const experimental = risk({
			methodology: { ...risk().methodology, validated: false },
		});
		render(<DiseaseDetail risk={experimental} today={TODAY} />);
		const method = screen
			.getByText("How this is calculated")
			.closest("details");
		const firstAfterSummary = method?.children[1];
		expect(firstAfterSummary?.textContent).toMatch(/^⚠ Experimental/);
	});

	// @spec DIS-UI-CHART-002
	it("shows the raised tier and its note while the meter and bars stay on the weather scale", () => {
		const raised = risk({
			tier: "Severe",
			tier_note:
				"Raised from High — recently overseeded turf is highly susceptible",
		});
		const { container } = render(<DiseaseDetail risk={raised} today={TODAY} />);
		expect(
			screen.getByText(
				"Raised from High — recently overseeded turf is highly susceptible",
			),
		).toBeInTheDocument();
		const badge = screen.getByText("E-index 5.4").nextElementSibling;
		expect(badge).toHaveTextContent("! Severe");
		expect((badge as HTMLElement).style.fontSize).toBe("0.9rem");
		const meterBands = Array.from(
			container.querySelector('[role="img"] > div')?.children ?? [],
		) as HTMLElement[];
		expect(meterBands.map((b) => b.style.opacity)).toEqual([
			"0.25",
			"0.25",
			"1",
			"0.25",
		]);
		// Today's bar keeps its own weather tier.
		const todayBar = container.querySelectorAll("path.recharts-rectangle")[1];
		expect(todayBar.getAttribute("fill")).toBe(RISK_TIER_COLORS.High);
	});

	// @spec DIS-UI-CONV-002
	it("reads each factor's status from the disease's side", () => {
		render(<DiseaseDetail risk={risk()} today={TODAY} />);
		const expected = [
			["Favorable for disease", "⚠", FACTOR_STATUS_COLORS.Favorable, "#d03b3b"],
			["Marginal for disease", "△", FACTOR_STATUS_COLORS.Marginal, "#fab219"],
			[
				"Unfavorable for disease",
				"✓",
				FACTOR_STATUS_COLORS.Unfavorable,
				"#0ca30c",
			],
		];
		for (const [label, symbol, color, pinned] of expected) {
			expect(color).toBe(pinned);
			const badge = screen.getByText(label).closest("span[data-badge]");
			expect(badge).toHaveTextContent(symbol);
			expect(badge).toHaveStyle({ borderColor: color });
		}
	});

	// @spec UI-CONV-001
	it("shows each factor's status as a badge: hidden symbol, visible words, tint and border", () => {
		render(<DiseaseDetail risk={risk()} today={TODAY} />);
		for (const [words, status] of [
			["Favorable for disease", "Favorable"],
			["Marginal for disease", "Marginal"],
			["Unfavorable for disease", "Unfavorable"],
		] as const) {
			const badge = screen.getByText(words).closest("span[data-badge]");
			expect(badge, words).not.toBeNull();
			const color = FACTOR_STATUS_COLORS[status];
			expect(badge).toHaveStyle({ borderColor: color, color: "#2d3748" });
			expect(badge?.getAttribute("style")).toContain("background-color");
			const symbol = badge?.querySelector("[aria-hidden='true']");
			expect(symbol, words).not.toBeNull();
		}
		expect(screen.queryByRole("img", { name: /for disease/ })).toBeNull();
	});

	// @spec DIS-UI-CONV-003
	it("takes every date from the response, never the browser clock", () => {
		vi.useFakeTimers({ toFake: ["Date"] });
		vi.setSystemTime(new Date("2031-03-02T03:30:00Z"));
		const yesterday = risk({
			as_of: "2026-09-19",
			management: management({
				action: "Protected",
				protection: {
					product: "Heritage G",
					class_label: "FRAC 11 (Strobilurins)",
					applied_on: "2026-09-01",
					protected_through: "2026-09-22",
					days_remaining: 2,
				},
			}),
		});
		render(<DiseaseDetail risk={yesterday} today={TODAY} />);
		expect(screen.getByText("Current risk · as of 9/19")).toBeInTheDocument();
		expect(
			screen.getByRole("heading", {
				name: "Contributing factors · as of 9/19",
			}),
		).toBeInTheDocument();
		expect(
			screen.getByText(/applied 9\/1 · covers through about 9\/22/),
		).toBeInTheDocument();
		const firstCells = screen
			.getAllByRole("row", { hidden: true })
			.map(
				(r) =>
					within(r).queryAllByRole("cell", { hidden: true })[0]?.textContent,
			);
		expect(firstCells).toEqual(
			expect.arrayContaining(["9/19", "Today", "9/21", "9/22"]),
		);
	});
});
