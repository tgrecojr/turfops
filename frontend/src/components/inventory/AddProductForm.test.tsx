// @vitest-environment jsdom
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { createProduct } from "../../api/client";
import { ApiError } from "../../api/errors";
import AddProductForm from "./AddProductForm";
import { makeProduct } from "./testFixtures";

vi.mock("../../api/client", () => ({ createProduct: vi.fn() }));
const create = vi.mocked(createProduct);

beforeEach(() => {
	create.mockClear();
});

function form() {
	const onCreated = vi.fn();
	const onError = vi.fn();
	render(<AddProductForm onCreated={onCreated} onError={onError} />);
	return { onCreated, onError };
}

// @spec INV-UI-002
describe("AddProductForm fields", () => {
	it("takes a required name, brand, category, form, stock, notes and the assistant box", () => {
		form();
		expect(screen.getByLabelText(/Product name/)).toBeRequired();
		for (const label of [/Brand/, /Category/, /^Form$/, /Stock/, /Notes/])
			expect(screen.getByLabelText(label)).toBeVisible();
		expect(screen.getByLabelText(/Identify with the assistant/)).toBeChecked();
		expect(screen.getByLabelText(/Category/)).not.toBeRequired();
		expect(screen.getByLabelText(/Category/)).toHaveDisplayValue(
			"Let the assistant decide",
		);
	});

	it("leaves the category to the assistant when it is on", async () => {
		const { onCreated } = form();
		create.mockResolvedValue({ product: makeProduct(), profile_error: null });
		await userEvent.type(screen.getByLabelText(/Product name/), "Heritage G");
		await userEvent.click(
			screen.getByRole("button", { name: "Add & identify" }),
		);
		expect(create).toHaveBeenCalledWith(
			expect.objectContaining({
				name: "Heritage G",
				category: undefined,
				assist: true,
			}),
		);
		expect(onCreated).toHaveBeenCalled();
	});

	it("requires a category before saving without the assistant", async () => {
		const { onError } = form();
		await userEvent.click(screen.getByLabelText(/Identify with the assistant/));
		expect(screen.getByLabelText(/Category \(required\)/)).toBeRequired();
		await userEvent.type(screen.getByLabelText(/Product name/), "Lime");
		const submit = screen.getByRole("button", { name: "Add product" });
		// Past the browser's own required check, the form still refuses.
		(submit.closest("form") as HTMLFormElement).noValidate = true;
		await userEvent.click(submit);
		expect(create).not.toHaveBeenCalled();
		expect(onError).toHaveBeenCalledWith(
			"Pick a category to save a product without the assistant.",
		);
	});
});

// @spec INV-UI-003
describe("AddProductForm while identifying", () => {
	it("says Identifying… and that it may take 10–30 seconds", async () => {
		form();
		create.mockReturnValue(new Promise(() => {}));
		expect(screen.getByText(/may take 10–30 seconds/)).toBeVisible();
		await userEvent.type(screen.getByLabelText(/Product name/), "Heritage G");
		await userEvent.click(
			screen.getByRole("button", { name: "Add & identify" }),
		);
		expect(screen.getByRole("button", { name: "Identifying…" })).toBeDisabled();
	});
});

// @spec INV-UI-013
describe("AddProductForm failure", () => {
	it("passes on the server's sentence, never a status code", async () => {
		const { onError } = form();
		create.mockRejectedValue(
			new ApiError(
				503,
				"The product assistant isn't configured. Pick a category.",
			),
		);
		await userEvent.type(screen.getByLabelText(/Product name/), "Heritage G");
		await userEvent.click(
			screen.getByRole("button", { name: "Add & identify" }),
		);
		expect(onError).toHaveBeenCalledWith(
			"The product assistant isn't configured. Pick a category.",
		);
	});

	it("points to Settings when there is no lawn profile", async () => {
		const { onError } = form();
		create.mockRejectedValue(new ApiError(404, "No lawn profile found"));
		await userEvent.type(screen.getByLabelText(/Product name/), "Heritage G");
		await userEvent.click(
			screen.getByRole("button", { name: "Add & identify" }),
		);
		expect(onError).toHaveBeenCalledWith(expect.stringMatching(/Settings/));
	});
});
