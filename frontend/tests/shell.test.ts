// Shell-wide rules the source must keep: one API client, no store or timer polling outside
// the two live pages, lazily loaded pages, and calendar dates that never pass through UTC.
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { describe, expect, it } from "vitest";

const root = join(import.meta.dirname, "..");
const src = join(root, "src");

/**
 * Every app source file under `dir`, as a path relative to `src`. Test support is not app
 * code: `*.test.*` suites, everything under `src/test/`, and `*Fixtures.ts` data builders.
 */
function sources(dir = src): string[] {
	return readdirSync(dir).flatMap((name) => {
		const path = join(dir, name);
		if (statSync(path).isDirectory())
			return path === join(src, "test") ? [] : sources(path);
		return /\.(ts|tsx)$/.test(name) &&
			!/\.test\./.test(name) &&
			!/Fixtures\.ts$/.test(name)
			? [relative(src, path)]
			: [];
	});
}

const text = (file: string) => readFileSync(join(src, file), "utf8");
/** The file without its comments, so prose about a pattern does not count as using it. */
const code = (file: string) =>
	text(file)
		.replace(/\/\*[\s\S]*?\*\//g, "")
		.replace(/^\s*\/\/.*$/gm, "");
const matching = (pattern: RegExp) =>
	sources().filter((file) => pattern.test(code(file)));

describe("API client", () => {
	// @spec UI-API-001
	it("is the only module that calls fetch, against the relative /api/v1 base", () => {
		expect(matching(/\bfetch\(/)).toEqual([join("api", "client.ts")]);
		expect(text(join("api", "client.ts"))).toContain('const BASE = "/api/v1";');
	});
});

describe("page data", () => {
	// @spec UI-SHELL-006
	it("is fetched by each page, with no store, cache library or polling elsewhere", () => {
		const pkg = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
		const deps = Object.keys({ ...pkg.dependencies, ...pkg.devDependencies });
		const stores =
			/redux|zustand|mobx|jotai|recoil|valtio|react-query|@tanstack\/|^swr$|apollo/;
		expect(deps.filter((d) => stores.test(d))).toEqual([]);
		expect(matching(/createContext|useContext\(/)).toEqual([]);
		expect(matching(/setInterval\(/).sort()).toEqual(
			[
				join("pages", "Dashboard.tsx"),
				join("pages", "Environmental.tsx"),
			].sort(),
		);
		const pages = sources(join(src, "pages"));
		expect(pages.length).toBe(12);
		for (const page of pages) {
			expect(text(page), page).toMatch(/useEffect\(/);
			expect(text(page), page).toMatch(/from "\.\.\/api\/client"/);
		}
	});
});

describe("page chunks", () => {
	// @spec UI-SHELL-003
	it("are lazy imports, with no page imported statically", () => {
		const app = text("App.tsx");
		const pages = readdirSync(join(src, "pages"))
			.filter((f) => /\.tsx$/.test(f) && !/\.test\./.test(f))
			.map((f) => f.replace(/\.tsx$/, ""));
		for (const page of pages) {
			expect(app, page).toContain(`lazy(() => import("./pages/${page}"))`);
		}
		expect(app).not.toMatch(/^import .* from "\.\/pages\//m);
		expect(app).toContain("<Suspense fallback={routeFallback}>");
	});
});

describe("calendar dates", () => {
	// @spec UI-UTIL-001
	it("are never taken from toISOString", () => {
		expect(matching(/toISOString\(\)/)).toEqual([]);
		for (const form of ["Applications.tsx", "SoilTests.tsx"]) {
			expect(code(join("pages", form)), form).toMatch(/todayLocalISO\(\)/);
		}
	});

	// @spec UI-UTIL-002
	it("from the API are never parsed as UTC midnight", () => {
		// A bare `YYYY-MM-DD` handed to Date is UTC midnight: the day before, west of UTC.
		expect(matching(/new Date\(\s*[\w.?]*date\s*\)/i)).toEqual([]);
		expect(matching(/Date\.parse\(\s*[\w.?]*date\s*\)/i)).toEqual([]);
	});
});

describe("type mirror", () => {
	// @spec UI-TYPE-001
	it("keeps shared models in types/index.ts and feature models in their own modules", () => {
		const shared = text(join("types", "index.ts"));
		for (const model of [
			"LawnProfile",
			"Application",
			"EnvironmentalSummary",
			"Recommendation",
			"DashboardResponse",
			"CalendarResponse",
			"GddSummary",
			"HistoricalData",
			"NitrogenBudget",
			"SoilTempForecast",
			"SeasonalPlan",
			"SoilTest",
			"Plant",
		]) {
			expect(shared, model).toMatch(
				new RegExp(`export interface ${model} \\{`),
			);
		}
		const features: Record<string, string[]> = {
			"disease.ts": ["DiseaseRiskResponse", "RiskTier"],
			"timing.ts": ["TimingResponse", "WindowState"],
			"inventory.ts": ["Product", "StockStatus", "InventoryStatus"],
		};
		for (const [file, models] of Object.entries(features)) {
			const feature = text(join("types", file));
			for (const model of models) {
				expect(feature, `${file} ${model}`).toMatch(
					new RegExp(`export (interface|type) ${model}\\b`),
				);
				expect(shared, `${model} lives in ${file}`).not.toMatch(
					new RegExp(`export (interface|type) ${model}\\b`),
				);
			}
		}
	});
});
