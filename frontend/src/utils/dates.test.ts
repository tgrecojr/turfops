import { describe, expect, it } from "vitest";
import { futureDateWarning } from "./dates";

describe("form dates against the server's today", () => {
	// @spec UI-UTIL-005
	it("warns about a date after the server's today, without blocking", () => {
		expect(futureDateWarning("2026-10-12", "2026-10-11")).toMatch(
			/after today/i,
		);
		expect(futureDateWarning("2026-10-11", "2026-10-11")).toBeNull();
		expect(futureDateWarning("2026-10-10", "2026-10-11")).toBeNull();
		expect(futureDateWarning("2026-10-12", null)).toBeNull();
	});
});
