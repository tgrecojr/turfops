// Repository-level checks: the HTML shell, CI, and conventions the source must keep.
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { describe, expect, it } from "vitest";

const root = join(import.meta.dirname, "..");
const src = join(root, "src");

function files(dir: string): string[] {
	return readdirSync(dir).flatMap((name) => {
		const path = join(dir, name);
		if (statSync(path).isDirectory()) return files(path);
		return /\.tsx$/.test(name) ? [path] : [];
	});
}

describe("app shell", () => {
	// @spec UI-SHELL-007
	it("titles the document TurfOps", () => {
		const html = readFileSync(join(root, "index.html"), "utf8");
		expect(html).toContain("<title>TurfOps</title>");
	});

	// @spec UI-BUILD-007
	it("type-checks the solution in CI with tsc -b", () => {
		const ci = readFileSync(
			join(root, "..", ".github", "workflows", "ci.yml"),
			"utf8",
		);
		expect(ci).toContain("npx tsc -b");
		expect(ci).not.toContain("tsc --noEmit");
	});
});

describe("conventions", () => {
	// @spec UI-CONV-003
	it("gives every chart a table view", () => {
		const charts = files(src).filter((path) =>
			readFileSync(path, "utf8").includes('from "recharts"'),
		);
		expect(charts.length).toBeGreaterThan(0);
		const missing = charts
			.filter((path) => {
				const text = readFileSync(path, "utf8");
				return (
					!text.includes("View as table") && !/import \w*Table\b/.test(text)
				);
			})
			.map((path) => relative(src, path));
		expect(missing).toEqual([]);
	});

	// @spec FEED-REC-004
	it("shows severity as symbol and label in ink on a tint everywhere", () => {
		for (const file of [
			"components/AlertCard.tsx",
			"pages/Recommendations.tsx",
			"components/inventory/ShoppingListPanel.tsx",
		]) {
			const text = readFileSync(join(src, file), "utf8");
			expect(text, file).toContain("severityChip(");
			expect(text, file).not.toMatch(/SEVERITY_COLORS\[/);
		}
	});
});

function sources(dir: string): string[] {
	return readdirSync(dir).flatMap((name) => {
		const path = join(dir, name);
		if (statSync(path).isDirectory()) return sources(path);
		return /\.(ts|tsx)$/.test(name) && !/\.test\./.test(name) ? [path] : [];
	});
}

describe("date display helpers", () => {
	// @spec UI-UTIL-006
	it("are defined once, in utils/dates.ts", () => {
		const root = src;
		const local =
			/function (format\w*Date\w*|formatMonthDay|formatMmDd|formatShortDate)\s*\(/;
		const offenders = sources(root)
			.filter((path) => !path.endsWith(join("utils", "dates.ts")))
			.filter((path) => local.test(readFileSync(path, "utf8")))
			.map((path) => relative(root, path));
		expect(offenders).toEqual([]);
	});
});
