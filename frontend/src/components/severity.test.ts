import { describe, expect, it } from "vitest";
import { SEVERITY_COLORS } from "../types";
import { severityChip } from "./severity";

describe("severity chip", () => {
	// @spec INV-UI-011
	it("reads as symbol and label in ink on a tint, never color alone", () => {
		expect(severityChip("Warning").text).toBe("⚠ Warning");
		expect(severityChip("Critical").text).toBe("! Critical");
		expect(severityChip("Advisory").text).toBe("→ Advisory");
		expect(severityChip("Info").text).toBe("ℹ Info");
		for (const s of ["Info", "Advisory", "Warning", "Critical"] as const) {
			const chip = severityChip(s);
			expect(chip.backgroundColor).toBe(`${SEVERITY_COLORS[s]}22`);
			expect(chip.color).not.toBe(SEVERITY_COLORS[s]);
			expect(chip.color).toMatch(/^#[0-9a-f]{6}$/);
		}
	});
});
