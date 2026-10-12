// @vitest-environment jsdom
import { act, fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import * as client from "../api/client";
import { ApiError } from "../api/errors";
import type { LawnProfile } from "../types";
import Settings from "./Settings";

vi.mock("../api/client", () => ({
	getProfile: vi.fn(),
	updateProfile: vi.fn(),
}));

const api = vi.mocked(client);

const PROFILE: LawnProfile = {
	id: 1,
	name: "Front",
	grass_type: "TallFescue",
	usda_zone: "7a",
	soil_type: "Clay",
	lawn_size_sqft: 5000,
	irrigation_type: "Hose",
	created_at: "2026-03-01T12:00:00Z",
	updated_at: "2026-03-02T12:00:00Z",
};

const SAVED: LawnProfile = { ...PROFILE, updated_at: "2026-10-11T18:45:00Z" };

beforeEach(() => {
	api.getProfile.mockResolvedValue(PROFILE);
	api.updateProfile.mockResolvedValue(SAVED);
});

afterEach(() => {
	vi.useRealTimers();
	vi.clearAllMocks();
});

describe("Settings", () => {
	// @spec APP-SET-001
	it("edits every profile field and clears the optional ones with one save", async () => {
		render(<Settings />);
		const name = await screen.findByLabelText("Lawn Name");
		expect(name).toBeRequired();
		expect(screen.getByLabelText("USDA Zone")).toBeRequired();
		expect(screen.getAllByRole("button")).toHaveLength(1);

		await userEvent.clear(name);
		await userEvent.type(name, "Back Forty");
		await userEvent.selectOptions(screen.getByLabelText("Grass Type"), "Mixed");
		await userEvent.clear(screen.getByLabelText("USDA Zone"));
		await userEvent.type(screen.getByLabelText("USDA Zone"), "6b");
		await userEvent.selectOptions(
			screen.getByLabelText("Soil Type"),
			"Not specified",
		);
		await userEvent.clear(screen.getByLabelText("Lawn Size (sqft)"));
		await userEvent.selectOptions(
			screen.getByLabelText("Irrigation Type"),
			"Not specified",
		);
		await userEvent.click(screen.getByRole("button", { name: "Save Profile" }));

		expect(api.updateProfile).toHaveBeenCalledWith({
			name: "Back Forty",
			grass_type: "Mixed",
			usda_zone: "6b",
			soil_type: "",
			lawn_size_sqft: 0,
			irrigation_type: "",
		});
	});

	// @spec APP-SET-002
	it("confirms the save for 3 seconds and shows when the profile was created and updated", async () => {
		vi.useFakeTimers();
		const settle = () =>
			act(async () => {
				await vi.advanceTimersByTimeAsync(0);
			});
		render(<Settings />);
		await settle();
		fireEvent.click(screen.getByRole("button", { name: "Save Profile" }));
		await settle();

		expect(screen.getByText("Profile saved successfully!")).toBeInTheDocument();
		const meta = screen.getByText(/Created:/);
		expect(meta).toHaveTextContent(
			new Date(SAVED.created_at).toLocaleDateString(),
		);
		expect(meta).toHaveTextContent(new Date(SAVED.updated_at).toLocaleString());

		await act(async () => {
			await vi.advanceTimersByTimeAsync(2999);
		});
		expect(screen.getByText("Profile saved successfully!")).toBeInTheDocument();
		await act(async () => {
			await vi.advanceTimersByTimeAsync(1);
		});
		expect(
			screen.queryByText("Profile saved successfully!"),
		).not.toBeInTheDocument();
	});

	// @spec APP-SET-003
	it("offers to create the profile when none exists", async () => {
		api.getProfile.mockRejectedValue(
			new ApiError(404, "No lawn profile found"),
		);
		render(<Settings />);

		expect(
			await screen.findByText(/There is no lawn profile yet/),
		).toHaveTextContent("saving creates it");
		expect(
			screen.queryByText(/Set one up in Settings/),
		).not.toBeInTheDocument();
		expect(screen.getByLabelText("Lawn Name")).toHaveValue("Main Lawn");
		expect(screen.getByLabelText("USDA Zone")).toHaveValue("7a");

		await userEvent.click(screen.getByRole("button", { name: "Save Profile" }));
		expect(api.updateProfile).toHaveBeenCalledWith(
			expect.objectContaining({ name: "Main Lawn", usda_zone: "7a" }),
		);
		expect(
			await screen.findByText("Profile saved successfully!"),
		).toBeInTheDocument();
		expect(
			screen.queryByText(/There is no lawn profile yet/),
		).not.toBeInTheDocument();
	});

	// @spec APP-LIST-005
	it("explains a failed load or save in plain words", async () => {
		api.getProfile.mockRejectedValueOnce(
			new ApiError(500, '{"error":"pool timed out"}'),
		);
		const { unmount } = render(<Settings />);
		const failed = await screen.findByText(
			/Something went wrong on the server/,
		);
		expect(failed).toHaveTextContent("Try again");
		expect(failed).not.toHaveTextContent("500");
		expect(failed).not.toHaveTextContent("pool timed out");
		unmount();

		api.updateProfile.mockRejectedValueOnce(
			new ApiError(400, "Unknown grass type: Bluegras"),
		);
		render(<Settings />);
		await userEvent.click(
			await screen.findByRole("button", { name: "Save Profile" }),
		);
		expect(
			await screen.findByText("Unknown grass type: Bluegras"),
		).toBeInTheDocument();
	});
});
