/// <reference types="vitest/config" />
import react from "@vitejs/plugin-react";
import { defineConfig, type Plugin } from "vite";

// recharts imports `es-toolkit/compat/<fn>` deep paths, whose export map only exposes
// the CommonJS build. rolldown-vite (vite 8) mis-compiles es-toolkit/compat's circular
// CJS modules into a broken init ("require_get is not a function", minified to
// "t is not a function") that crashes any page rendering a chart. The ESM barrel
// `es-toolkit/compat` (index.mjs) bundles cleanly, so redirect each deep default import
// to a named re-export of the barrel.
// @spec UI-BUILD-004
function esToolkitCompatEsm(): Plugin {
	const PREFIX = "es-toolkit/compat/";
	const VIRTUAL = "\0es-toolkit-compat-esm:";
	return {
		name: "es-toolkit-compat-esm",
		enforce: "pre",
		resolveId(id) {
			// Redirect deep subpaths (e.g. es-toolkit/compat/get) but not the barrel itself.
			if (id.startsWith(PREFIX)) return VIRTUAL + id.slice(PREFIX.length);
			return null;
		},
		load(id) {
			if (id.startsWith(VIRTUAL)) {
				const name = id.slice(VIRTUAL.length);
				return `export { ${name} as default } from 'es-toolkit/compat';`;
			}
			return null;
		},
	};
}

// @spec UI-BUILD-001
export default defineConfig({
	plugins: [esToolkitCompatEsm(), react()],
	// @spec UI-BUILD-005
	server: {
		port: 5173,
		proxy: {
			"/api": {
				target: "http://localhost:3000",
				changeOrigin: true,
			},
		},
	},
	// @spec UI-BUILD-008, UI-BUILD-010
	test: {
		// Component tests opt into a DOM with a `// @vitest-environment jsdom` docblock;
		// everything else runs in Node.
		setupFiles: ["./src/test/setup.ts"],
	},
});
