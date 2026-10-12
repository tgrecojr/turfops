import { afterEach, describe, expect, it, vi } from "vitest";
import * as client from "./client";
import { ApiError } from "./errors";

/** A fetch that never answers on its own: it settles only when its signal aborts. */
function hangingFetch() {
	const spy = vi.fn(
		(_url: string, init?: RequestInit) =>
			new Promise<Response>((_resolve, reject) => {
				init?.signal?.addEventListener("abort", () =>
					reject(new DOMException("aborted", "AbortError")),
				);
			}),
	);
	vi.stubGlobal("fetch", spy);
	return spy;
}

function okFetch() {
	const spy = vi.fn(
		async () =>
			new Response("{}", {
				status: 200,
				headers: { "Content-Type": "application/json" },
			}),
	);
	vi.stubGlobal("fetch", spy);
	return spy;
}

const urlOf = (spy: ReturnType<typeof okFetch>) =>
	String((spy.mock.calls[0] as unknown[])[0]);

afterEach(() => {
	vi.unstubAllGlobals();
	vi.useRealTimers();
});

/** Whether `call` is still pending just before `ms`, and timed out at `ms`. */
async function timesOutAt(call: () => Promise<unknown>, ms: number) {
	vi.useFakeTimers();
	hangingFetch();
	let settled: unknown = "pending";
	const done = call().then(
		() => "resolved",
		(e: unknown) => e,
	);
	void done.then((v) => {
		settled = v;
	});
	await vi.advanceTimersByTimeAsync(ms - 1);
	const before = settled;
	await vi.advanceTimersByTimeAsync(1);
	const after = await done;
	return {
		pendingBefore: before === "pending",
		timedOut: after instanceof ApiError && after.timedOut,
	};
}

describe("request timeouts", () => {
	// @spec UI-API-002
	it.each([
		["the seasonal plan", () => client.getSeasonalPlan(), 30_000],
		["the shopping list", () => client.getShoppingList(), 30_000],
		[
			"creating a plant",
			() => client.createPlant({ plant_type: "Shrub", common_name: "Holly" }),
			60_000,
		],
		["refreshing a care plan", () => client.refreshPlantPlan(1), 60_000],
		[
			"creating a product",
			() => client.createProduct({ name: "Prodiamine" }),
			60_000,
		],
		[
			"refreshing a product profile",
			() => client.refreshProductProfile(1),
			60_000,
		],
		["the dashboard", () => client.getDashboard(), 15_000],
		["the profile", () => client.getProfile(), 15_000],
		["deleting a product", () => client.deleteProduct(1), 15_000],
	] as const)("aborts %s after its own timeout", async (_name, call, ms) => {
		expect(await timesOutAt(call, ms)).toEqual({
			pendingBefore: true,
			timedOut: true,
		});
	});
});

describe("request paths", () => {
	// @spec UI-API-001
	it("call the relative /api/v1 base", async () => {
		const spy = okFetch();
		await client.getDashboard();
		expect(urlOf(spy)).toBe("/api/v1/dashboard");
	});

	// @spec UI-API-004
	it("URI-encode a recommendation id taken from data", async () => {
		const spy = okFetch();
		await client.patchRecommendation("plant_12/pruning 2026?x#y", {
			dismissed: true,
		});
		expect(urlOf(spy)).toBe(
			"/api/v1/recommendations/plant_12%2Fpruning%202026%3Fx%23y",
		);
	});

	// @spec UI-API-004
	it("URI-encode a product name typed by the user", async () => {
		const spy = okFetch();
		await client.getFracClasses("Heritage & Daconil 50%");
		expect(urlOf(spy)).toBe(
			"/api/v1/frac-classes?product=Heritage%20%26%20Daconil%2050%25",
		);
	});
});
