// Shared Vitest setup. Node-environment tests skip the DOM half.
// @spec UI-BUILD-010
import { afterEach } from "vitest";

if (typeof window !== "undefined") {
	await import("@testing-library/jest-dom/vitest");
	const { cleanup } = await import("@testing-library/react");
	afterEach(() => cleanup());

	// jsdom has no layout: Recharts' ResponsiveContainer needs ResizeObserver.
	if (!("ResizeObserver" in window)) {
		class ResizeObserverStub {
			observe() {}
			unobserve() {}
			disconnect() {}
		}
		Object.assign(window, { ResizeObserver: ResizeObserverStub });
	}
	if (!window.matchMedia) {
		window.matchMedia = (query: string) =>
			({
				matches: false,
				media: query,
				onchange: null,
				addListener() {},
				removeListener() {},
				addEventListener() {},
				removeEventListener() {},
				dispatchEvent: () => false,
			}) as MediaQueryList;
	}
}
