import { afterEach, describe, expect, it, vi } from "vitest";
import { fetchJson } from "./client";
import { ApiError } from "./errors";

function respond(status: number, body: string, type = "application/json") {
	vi.stubGlobal(
		"fetch",
		vi.fn(
			async () =>
				new Response(body || null, {
					status,
					headers: { "Content-Type": type },
				}),
		),
	);
}

afterEach(() => {
	vi.unstubAllGlobals();
	vi.useRealTimers();
});

// @spec UI-ERR-004
describe("fetchJson errors", () => {
	it("keeps the status and unwraps the server's error message", async () => {
		respond(
			409,
			JSON.stringify({ error: "An identical entry is already logged." }),
		);
		const err = await fetchJson<never>("/api/v1/x").catch((e: ApiError) => e);
		expect(err).toBeInstanceOf(ApiError);
		expect(err.status).toBe(409);
		expect(err.message).toBe("An identical entry is already logged.");
	});

	it("never carries a raw non-JSON body", async () => {
		respond(502, "<html>Bad Gateway from proxy</html>", "text/html");
		const err = await fetchJson<never>("/api/v1/x").catch((e: ApiError) => e);
		expect(err).toBeInstanceOf(ApiError);
		expect(err.status).toBe(502);
		expect(err.message).not.toMatch(/<html>/);
	});

	it("marks a request its timeout cancelled", async () => {
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
		const err = await fetchJson<never>("/api/v1/x", undefined, 10).catch(
			(e: ApiError) => e,
		);
		expect(err).toBeInstanceOf(ApiError);
		expect(err.timedOut).toBe(true);
	});
});

// @spec UI-ERR-008
describe("fetchJson cancellation", () => {
	it("passes a caller's own cancellation through instead of calling it a timeout", async () => {
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
		const caller = new AbortController();
		const pending = fetchJson<never>("/api/v1/x", {
			signal: caller.signal,
		}).catch((e: unknown) => e);
		caller.abort();
		const err = await pending;
		expect(err).not.toBeInstanceOf(ApiError);
		expect((err as DOMException).name).toBe("AbortError");
	});
});

// @spec UI-API-003
describe("fetchJson success", () => {
	it("resolves an empty 2xx body as no value", async () => {
		respond(200, "");
		await expect(fetchJson("/api/v1/x")).resolves.toBeUndefined();
	});

	it("parses a JSON body", async () => {
		respond(200, JSON.stringify({ ok: true }));
		await expect(fetchJson("/api/v1/x")).resolves.toEqual({ ok: true });
	});
});

describe("applications list", () => {
	// @spec APP-LIST-003
	it("asks the server for the type and scope", async () => {
		const fetchSpy = vi.fn(
			async () =>
				new Response("[]", {
					status: 200,
					headers: { "Content-Type": "application/json" },
				}),
		);
		vi.stubGlobal("fetch", fetchSpy);
		const { getApplications } = await import("./client");
		await getApplications("Fertilizer", 50, 0, "landscape");
		const url = String((fetchSpy.mock.calls[0] as unknown[])[0]);
		expect(url).toContain("type=Fertilizer");
		expect(url).toContain("scope=landscape");
		expect(url).toContain("offset=0");
	});
});

describe("request options", () => {
	// @spec UI-API-005
	it("merges a caller's headers with the JSON default", async () => {
		const fetchSpy = vi.fn(
			async () =>
				new Response("{}", {
					status: 200,
					headers: { "Content-Type": "application/json" },
				}),
		);
		vi.stubGlobal("fetch", fetchSpy);
		await fetchJson("/api/v1/x", { headers: { "X-Trace": "1" } });
		const init = (fetchSpy.mock.calls[0] as unknown[])[1] as RequestInit;
		const headers = new Headers(init.headers);
		expect(headers.get("Content-Type")).toBe("application/json");
		expect(headers.get("X-Trace")).toBe("1");
	});

	// @spec UI-API-006
	it("keeps the timeout when the caller passes its own signal, and honours that signal", async () => {
		vi.useFakeTimers();
		vi.stubGlobal(
			"fetch",
			vi.fn(
				(_url: string, init: RequestInit) =>
					new Promise((_, reject) => {
						init.signal?.addEventListener("abort", () =>
							reject(new DOMException("aborted", "AbortError")),
						);
					}),
			),
		);
		const caller = new AbortController();
		const timedOut = fetchJson(
			"/api/v1/x",
			{ signal: caller.signal },
			1000,
		).catch((e: ApiError) => e);
		await vi.advanceTimersByTimeAsync(1000);
		const err = await timedOut;
		expect(err).toBeInstanceOf(ApiError);
		expect((err as ApiError).timedOut).toBe(true);

		const mine = new AbortController();
		const cancelled = fetchJson(
			"/api/v1/x",
			{ signal: mine.signal },
			1000,
		).catch((e: unknown) => e);
		mine.abort();
		const abort = await cancelled;
		expect(abort).toBeInstanceOf(DOMException);
	});
});

describe("server today", () => {
	// @spec UI-UTIL-005, DEPLOY-TZ-006
	it("reads the lawn's date from /api/v1/today", async () => {
		const fetchSpy = vi.fn(
			async () =>
				new Response(
					JSON.stringify({ today: "2026-10-11", timezone: "America/New_York" }),
					{ status: 200, headers: { "Content-Type": "application/json" } },
				),
		);
		vi.stubGlobal("fetch", fetchSpy);
		const { getToday } = await import("./client");
		expect(await getToday()).toEqual({
			today: "2026-10-11",
			timezone: "America/New_York",
		});
		expect(String((fetchSpy.mock.calls[0] as unknown[])[0])).toBe(
			"/api/v1/today",
		);
	});
});
