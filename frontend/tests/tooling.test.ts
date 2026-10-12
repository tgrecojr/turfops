// The build, lint, type-check and test tooling, checked from the files that configure it.
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import type { Plugin } from "vite";
import { describe, expect, it } from "vitest";
import config from "../vite.config";

const root = join(import.meta.dirname, "..");
const read = (path: string) => readFileSync(join(root, path), "utf8");
const pkg = JSON.parse(read("package.json")) as {
	scripts: Record<string, string>;
	dependencies: Record<string, string>;
	devDependencies: Record<string, string>;
};

/** A tsconfig file's JSON, with its comments removed. */
function tsconfig(path: string): {
	compilerOptions?: Record<string, unknown>;
} {
	const text = read(path).replace(/\/\*[\s\S]*?\*\//g, "");
	return JSON.parse(text.replace(/^\s*\/\/.*$/gm, ""));
}

/** The CI workflow's frontend job (up to the next top-level job). */
function frontendJob(): string {
	const ci = readFileSync(
		join(root, "..", ".github", "workflows", "ci.yml"),
		"utf8",
	);
	const start = ci.indexOf("\n  frontend:");
	const rest = ci.slice(start + 1);
	const next = rest.slice(1).search(/\n {2}\S/);
	return next === -1 ? rest : rest.slice(0, next + 1);
}

function ciTriggers(): string {
	const ci = readFileSync(
		join(root, "..", ".github", "workflows", "ci.yml"),
		"utf8",
	);
	return ci.slice(ci.indexOf("\non:"), ci.indexOf("\njobs:"));
}

/** The configured Vite plugins, in order, nested plugin arrays flattened. */
function plugins(): Plugin[] {
	const flat = (config.plugins ?? []).flat(3) as unknown[];
	return flat.filter(
		(p): p is Plugin => !!p && typeof p === "object" && "name" in p,
	);
}

function plugin(name: string): Plugin {
	const found = plugins().find((p) => p.name === name);
	if (!found) throw new Error(`no ${name} plugin`);
	return found;
}

type Hook = (id: string) => string | null;

describe("production build", () => {
	// @spec UI-BUILD-001
	it("type-checks the whole solution before bundling into dist", () => {
		expect(pkg.scripts.build).toBe("tsc -b && vite build");
		// No outDir override: Vite writes the bundle to frontend/dist.
		expect(config.build?.outDir).toBeUndefined();
		const solution = tsconfig("tsconfig.json") as {
			files?: string[];
			references?: { path: string }[];
		};
		expect(solution.files).toEqual([]);
		expect(solution.references?.map((r) => r.path)).toEqual([
			"./tsconfig.app.json",
			"./tsconfig.node.json",
		]);
	});

	// @spec UI-BUILD-002
	it("compiles the app strictly", () => {
		const options = tsconfig("tsconfig.app.json").compilerOptions ?? {};
		for (const flag of [
			"strict",
			"noUnusedLocals",
			"noUnusedParameters",
			"erasableSyntaxOnly",
			"verbatimModuleSyntax",
		]) {
			expect(options[flag], flag).toBe(true);
		}
	});

	// @spec UI-BUILD-004
	it("redirects es-toolkit/compat deep imports to the ESM barrel, before Vite resolves", () => {
		const redirect = plugin("es-toolkit-compat-esm");
		expect(redirect.enforce).toBe("pre");
		expect(plugins()[0].name).toBe("es-toolkit-compat-esm");

		const resolveId = redirect.resolveId as unknown as Hook;
		const load = redirect.load as unknown as Hook;
		const id = resolveId("es-toolkit/compat/get");
		expect(id).not.toBeNull();
		expect(load(id as string)).toBe(
			"export { get as default } from 'es-toolkit/compat';",
		);
		// The barrel itself, and anything else, is left to Vite.
		expect(resolveId("es-toolkit/compat")).toBeNull();
		expect(resolveId("recharts")).toBeNull();
	});
});

describe("dev server", () => {
	// @spec UI-BUILD-005
	it("runs on 5173 and proxies /api to the backend on 3000", () => {
		expect(config.server?.port).toBe(5173);
		const proxy = config.server?.proxy?.["/api"];
		expect(typeof proxy === "object" ? proxy.target : proxy).toBe(
			"http://localhost:3000",
		);
	});
});

describe("lint", () => {
	// @spec UI-BUILD-003
	it("runs Biome's recommended rules with the React domain and the format check", () => {
		expect(pkg.scripts.lint).toBe("biome check .");
		const text = read("biome.jsonc");
		expect(text).toContain("@spec UI-BUILD-003");
		const json = text
			.split("\n")
			.filter((line) => !line.trim().startsWith("//"))
			.join("\n");
		const biome = JSON.parse(json) as {
			files: { includes: string[] };
			linter: {
				rules?: { recommended?: boolean; style?: Record<string, string> };
				domains: Record<string, string>;
			};
			formatter?: { enabled?: boolean };
		};
		expect(biome.linter.domains.react).toBe("recommended");
		expect(biome.linter.rules?.recommended).not.toBe(false);
		expect(biome.linter.rules?.style?.useComponentExportOnlyModules).toBe(
			"error",
		);
		expect(biome.formatter?.enabled).not.toBe(false);
		expect(biome.files.includes).toEqual(["**", "!dist", "!public"]);
	});
});

describe("tests", () => {
	// @spec UI-BUILD-008
	it("run with Vitest over suites that cover the client's error mapping", () => {
		expect(pkg.scripts.test).toBe("vitest run");
		expect(pkg.devDependencies.vitest).toBeDefined();
		const client = read("src/api/client.test.ts");
		expect(client).toContain("@spec UI-ERR-004");
		const errors = read("src/api/errors.test.ts");
		expect(errors).toMatch(/too long/);
	});

	// @spec UI-BUILD-010
	it("run in Node unless a component test opts into jsdom with its docblock", () => {
		expect(config.test?.environment ?? "node").toBe("node");
		expect(config.test?.setupFiles).toEqual(["./src/test/setup.ts"]);
		const components = componentTests(join(root, "src"));
		expect(components.length).toBeGreaterThan(0);
		for (const file of components) {
			expect(readFileSync(file, "utf8"), file).toMatch(
				/^\/\/ @vitest-environment jsdom\n/,
			);
		}
		for (const lib of [
			"@testing-library/react",
			"@testing-library/user-event",
			"@testing-library/jest-dom",
			"jsdom",
		]) {
			expect(pkg.devDependencies[lib], lib).toBeDefined();
		}
	});
});

/** Every `*.test.tsx` under `dir`. */
function componentTests(dir: string): string[] {
	return readdirSync(dir).flatMap((name) => {
		const path = join(dir, name);
		if (statSync(path).isDirectory()) return componentTests(path);
		return name.endsWith(".test.tsx") ? [path] : [];
	});
}

describe("CI frontend job", () => {
	// @spec UI-BUILD-006, UI-BUILD-009
	it("lints and tests on every push to and pull request against main", () => {
		const triggers = ciTriggers();
		expect(triggers).toMatch(/push:\s*\n\s*branches: \[main\]/);
		expect(triggers).toMatch(/pull_request:\s*\n\s*branches: \[main\]/);
		const job = frontendJob();
		expect(job).toContain("working-directory: frontend");
		expect(job).toMatch(/run: npm run lint\n/);
		expect(job).toMatch(/run: npm test\n/);
	});
});
