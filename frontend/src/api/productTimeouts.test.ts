import { afterEach, describe, expect, it, vi } from "vitest";
import { createProduct, refreshProductProfile } from "./client";
import { ApiError } from "./errors";

/** A fetch that never answers, so only the client's timeout ends it. */
function hang() {
	vi.stubGlobal(
		"fetch",
		vi.fn(
			(_url: string, init?: RequestInit) =>
				new Promise((_resolve, reject) => {
					init?.signal?.addEventListener("abort", () =>
						reject(new DOMException("aborted", "AbortError")),
					);
				}),
		),
	);
}

afterEach(() => {
	vi.unstubAllGlobals();
	vi.useRealTimers();
});

// @spec INV-UI-003
describe("assistant round-trips", () => {
	it.each([
		["an add", () => createProduct({ name: "Heritage G" })],
		["a profile refresh", () => refreshProductProfile(1)],
	])("wait up to 60 seconds for %s before timing out", async (_, call) => {
		vi.useFakeTimers();
		hang();
		let settled: ApiError | null = null;
		call().catch((e: ApiError) => {
			settled = e;
		});
		await vi.advanceTimersByTimeAsync(59_999);
		expect(settled).toBeNull();
		await vi.advanceTimersByTimeAsync(1);
		expect(settled).toBeInstanceOf(ApiError);
		expect((settled as unknown as ApiError).timedOut).toBe(true);
	});
});
