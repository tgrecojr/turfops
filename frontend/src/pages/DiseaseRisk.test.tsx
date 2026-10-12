// @vitest-environment jsdom
import { render, screen, within } from "@testing-library/react";
import { cloneElement, type ReactElement } from "react";
import { MemoryRouter, Route, Routes } from "react-router";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { getDiseaseRisk } from "../api/client";
import { ApiError } from "../api/errors";
import { response } from "../components/disease/testFixtures";
import DiseaseRisk from "./DiseaseRisk";

vi.mock("../api/client", () => ({ getDiseaseRisk: vi.fn() }));
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

const fetchRisk = vi.mocked(getDiseaseRisk);

function show(path: string) {
	return render(
		<MemoryRouter initialEntries={[path]}>
			<Routes>
				<Route path="/disease-risk" element={<DiseaseRisk />} />
				<Route path="/disease-risk/:slug" element={<DiseaseRisk />} />
			</Routes>
		</MemoryRouter>,
	);
}

function before(a: Element, b: Element): boolean {
	return Boolean(
		a.compareDocumentPosition(b) & Node.DOCUMENT_POSITION_FOLLOWING,
	);
}

beforeEach(() => {
	fetchRisk.mockReset();
});

describe("Disease Risk page", () => {
	// @spec DIS-UI-PAGE-001
	it("lists station, notes, the explainer and one linked row per disease", async () => {
		fetchRisk.mockResolvedValue(
			response({ data_notes: ["Note one.", "Note two."] }),
		);
		show("/disease-risk");
		const station = await screen.findByText("NOAA USCRN station 3761");
		const notes = [
			screen.getByText("Note one."),
			screen.getByText("Note two."),
		];
		const explainer = screen.getByText(/there is no blended score/);
		expect(explainer).toHaveTextContent(/native scores\s+are not comparable/);
		const rows = screen.getAllByRole("link");
		expect(rows.map((r) => r.getAttribute("href"))).toEqual([
			"/disease-risk/brown-patch",
			"/disease-risk/dollar-spot",
		]);
		const first = within(rows[0]);
		expect(first.getByText("Brown Patch")).toBeInTheDocument();
		expect(
			first.getByText("E-index 5.4 · Fidanza-Dernoeden E-index"),
		).toBeInTheDocument();
		expect(
			first.getByRole("img", { name: "Risk meter: High" }),
		).toBeInTheDocument();
		expect(rows[0]).toHaveTextContent("⚠ High");
		for (const [a, b] of [
			[station, notes[0]],
			[notes[1], explainer],
			[explainer, rows[0]],
		]) {
			expect(before(a, b)).toBe(true);
		}
	});

	// @spec DIS-UI-PAGE-002
	it("with no diseases shows the first data note, or a fallback", async () => {
		fetchRisk.mockResolvedValue(
			response({
				diseases: [],
				data_notes: ["Disease models cover cool-season turf only."],
			}),
		);
		const { unmount } = show("/disease-risk");
		expect(
			await screen.findAllByText("Disease models cover cool-season turf only."),
		).toHaveLength(2);
		unmount();

		fetchRisk.mockResolvedValue(response({ diseases: [], data_notes: [] }));
		show("/disease-risk");
		expect(
			await screen.findByText(
				"Not enough recent weather data to score disease risk.",
			),
		).toBeInTheDocument();
	});

	// @spec DIS-UI-PAGE-003
	it("shows one disease's detail with a way back, or says it has no data", async () => {
		fetchRisk.mockResolvedValue(response());
		const { unmount } = show("/disease-risk/brown-patch");
		expect(
			await screen.findByText("NOAA USCRN station 3761"),
		).toBeInTheDocument();
		expect(
			screen.getByText("Forecast unavailable — no outlook days are shown."),
		).toBeInTheDocument();
		expect(
			screen.getByRole("link", { name: "← All diseases" }),
		).toHaveAttribute("href", "/disease-risk");
		expect(
			screen.getByRole("heading", { name: "Brown Patch Risk" }),
		).toBeInTheDocument();
		expect(screen.getByText("How this is calculated")).toBeInTheDocument();
		unmount();

		show("/disease-risk/snow-mold");
		expect(
			await screen.findByText(/No risk data for “snow-mold”/),
		).toBeInTheDocument();
		expect(
			screen.getByRole("link", { name: "Back to all diseases" }),
		).toHaveAttribute("href", "/disease-risk");
	});

	// @spec DIS-UI-PAGE-004
	it("says it is loading until the response arrives", () => {
		fetchRisk.mockReturnValue(new Promise(() => {}));
		show("/disease-risk");
		expect(screen.getByText("Loading disease risk…")).toBeInTheDocument();
	});

	// @spec DIS-UI-PAGE-005
	it.each([
		[
			new ApiError(404, "No lawn profile found"),
			"There is no lawn profile yet. Set one up in Settings to get started.",
		],
		[
			new ApiError(
				503,
				"The weather data lake isn't configured, so disease risk can't be scored.",
			),
			"The weather data lake isn't configured, so disease risk can't be scored.",
		],
		[
			new ApiError(500, '{"error":"duckdb: IO Error"}'),
			"Something went wrong on the server. Try again; the details are in the server log.",
		],
	])("turns a failure into a plain sentence (%s)", async (err, sentence) => {
		fetchRisk.mockRejectedValue(err);
		show("/disease-risk/brown-patch");
		expect(await screen.findByText(sentence)).toBeInTheDocument();
		expect(document.body).not.toHaveTextContent(
			/duckdb|\{"error"|\b50\d\b|\b404\b/,
		);
	});
});
