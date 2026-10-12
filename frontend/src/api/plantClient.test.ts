import { afterEach, describe, expect, it, vi } from "vitest";
import { createPlant, refreshPlantPlan } from "./client";
import { ApiError } from "./errors";

/** A server that never answers until the request is cancelled. */
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

async function waitsSixtySeconds(send: () => Promise<unknown>) {
	vi.useFakeTimers();
	hang();
	let settled: unknown = "pending";
	send().catch((e) => {
		settled = e;
	});
	await vi.advanceTimersByTimeAsync(59_999);
	expect(settled).toBe("pending");
	await vi.advanceTimersByTimeAsync(1);
	expect(settled).toBeInstanceOf(ApiError);
	expect((settled as ApiError).timedOut).toBe(true);
}

// @spec PLANT-UI-004
describe("plan generation requests", () => {
	it("adding a plant waits up to 60 s", async () => {
		await waitsSixtySeconds(() =>
			createPlant({ common_name: "Holly", plant_type: "Shrub" }),
		);
	});

	it("regenerating a plan waits up to 60 s", async () => {
		await waitsSixtySeconds(() => refreshPlantPlan(4));
	});
});
