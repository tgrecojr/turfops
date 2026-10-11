import { describe, expect, it } from "vitest";
import { loadDashboard } from "./loadDashboard";

type W = { gdd: number; timing: string };

describe("dashboard poll", () => {
	// @spec UI-DASH-003
	it("hands every request the poll's abort signal", async () => {
		const seen: (AbortSignal | undefined)[] = [];
		const record =
			<T>(v: T) =>
			async (signal?: AbortSignal) => {
				seen.push(signal);
				return v;
			};
		const controller = new AbortController();
		await loadDashboard<string, W>(
			{
				dashboard: record("d"),
				widgets: { gdd: record(1), timing: record("t") },
			},
			controller.signal,
			null,
		);
		expect(seen).toHaveLength(3);
		expect(seen.every((s) => s === controller.signal)).toBe(true);
	});

	// @spec FEED-UI-013
	it("keeps a widget's last good value, marked stale, when its re-poll fails", async () => {
		const failing = async () => {
			throw new Error("timeout");
		};
		const first = await loadDashboard<string, W>(
			{
				dashboard: async () => "d",
				widgets: { gdd: async () => 120, timing: async () => "ok" },
			},
			new AbortController().signal,
			null,
		);
		expect(first.widgets.gdd).toEqual({ value: 120, stale: false });
		const second = await loadDashboard<string, W>(
			{
				dashboard: async () => "d",
				widgets: { gdd: failing, timing: async () => "new" },
			},
			new AbortController().signal,
			first.widgets,
		);
		expect(second.widgets.gdd).toEqual({ value: 120, stale: true });
		expect(second.widgets.timing).toEqual({ value: "new", stale: false });
		const never = await loadDashboard<string, W>(
			{
				dashboard: async () => "d",
				widgets: { gdd: failing, timing: failing },
			},
			new AbortController().signal,
			null,
		);
		expect(never.widgets.gdd).toEqual({ value: null, stale: false });
	});
});
