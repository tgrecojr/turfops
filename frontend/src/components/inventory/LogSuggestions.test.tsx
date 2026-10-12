// @vitest-environment jsdom
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { listLoggedProductSuggestions } from "../../api/client";
import type { LoggedProductSuggestion } from "../../types/inventory";
import LogSuggestions from "./LogSuggestions";

vi.mock("../../api/client", () => ({ listLoggedProductSuggestions: vi.fn() }));
const fetchSuggestions = vi.mocked(listLoggedProductSuggestions);

const grubex: LoggedProductSuggestion = {
	name: "GrubEx",
	uses: 3,
	last_used: "2026-06-01",
	application_types: ["GrubControl", "Insecticide"],
};
const turfBuilder: LoggedProductSuggestion = {
	name: "Turf Builder",
	uses: 1,
	last_used: "2026-04-01",
	application_types: ["Fertilizer"],
};

beforeEach(() => {
	fetchSuggestions.mockClear();
});

// @spec INV-LOG-003
describe("LogSuggestions", () => {
	it("lists logged names with uses, last date and types", async () => {
		fetchSuggestions.mockResolvedValue([grubex, turfBuilder]);
		render(<LogSuggestions onAdd={vi.fn()} />);
		expect(await screen.findByText("From your application log")).toBeVisible();
		expect(
			screen.getByText(/3× · last 2026-06-01 · Grub Control, Insecticide/),
		).toBeVisible();
	});

	it("hands Add to shelf the suggestion and hides one on Not now", async () => {
		fetchSuggestions.mockResolvedValue([grubex, turfBuilder]);
		const onAdd = vi.fn();
		render(<LogSuggestions onAdd={onAdd} />);
		const addButtons = await screen.findAllByRole("button", {
			name: "Add to shelf",
		});
		await userEvent.click(addButtons[0]);
		expect(onAdd).toHaveBeenCalledWith(grubex);
		await userEvent.click(
			screen.getAllByRole("button", { name: "Not now" })[0],
		);
		expect(screen.queryByText("GrubEx")).toBeNull();
		expect(screen.getByText("Turf Builder")).toBeVisible();
	});

	it("is hidden when the list is empty or its fetch fails", async () => {
		fetchSuggestions.mockResolvedValue([]);
		const empty = render(<LogSuggestions onAdd={vi.fn()} />);
		await vi.waitFor(() => expect(fetchSuggestions).toHaveBeenCalled());
		expect(empty.container).toBeEmptyDOMElement();
		empty.unmount();

		fetchSuggestions.mockRejectedValue(new Error("boom"));
		const failed = render(<LogSuggestions onAdd={vi.fn()} />);
		await vi.waitFor(() => expect(fetchSuggestions).toHaveBeenCalledTimes(2));
		expect(failed.container).toBeEmptyDOMElement();
	});
});
