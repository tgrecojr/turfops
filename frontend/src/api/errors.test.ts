import { describe, expect, it } from "vitest";
import { ApiError, describeError } from "./errors";

// @spec UI-ERR-005, UI-ERR-006, UI-ERR-007, UI-ERR-009, UI-ERR-010
describe("describeError", () => {
	it("points a missing lawn profile to Settings", () => {
		const msg = describeError(new ApiError(404, "No lawn profile found"));
		expect(msg).toMatch(/Settings/);
		expect(msg).not.toMatch(/404/);
	});

	it("shows a 503's own plain message, which names the source", () => {
		const server =
			"The weather data lake can't be read right now, so soil, disease, timing and history data are unavailable. Try again in a few minutes.";
		expect(describeError(new ApiError(503, server))).toBe(server);
	});

	it("shows a 400 or 409 message as written, with no status prefix", () => {
		expect(
			describeError(new ApiError(409, "An identical entry is already logged.")),
		).toBe("An identical entry is already logged.");
		expect(
			describeError(new ApiError(400, "Pick a plant for this application.")),
		).toBe("Pick a plant for this application.");
	});

	it("says a server error is on the server and to try again", () => {
		const msg = describeError(new ApiError(500, "anything"));
		expect(msg).toMatch(/server/i);
		expect(msg).toMatch(/try again/i);
		expect(msg).not.toMatch(/500/);
	});

	it("says a timeout took too long and to try again", () => {
		const msg = describeError(new ApiError(0, "aborted", true));
		expect(msg).toMatch(/too long/i);
		expect(msg).toMatch(/try again/i);
	});

	it("says TurfOps could not be reached when no response arrived", () => {
		const msg = describeError(new TypeError("Failed to fetch"));
		expect(msg).toMatch(/reach/i);
		expect(msg).not.toMatch(/Failed to fetch/);
	});
});
