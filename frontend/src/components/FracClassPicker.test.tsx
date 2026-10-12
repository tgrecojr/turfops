// @vitest-environment jsdom
import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import * as client from "../api/client";
import FracClassPicker from "./FracClassPicker";

vi.mock("../api/client", () => ({ getFracClasses: vi.fn() }));

const api = vi.mocked(client);

const OPTIONS = [
	{
		id: "Frac3",
		label: "FRAC 3 — DMIs",
		examples: ["Banner Maxx", "Bayleton"],
	},
	{ id: "Frac11", label: "FRAC 11 — Strobilurins", examples: ["Heritage G"] },
];

beforeEach(() => {
	vi.useFakeTimers();
	api.getFracClasses.mockImplementation(async (product?: string) => ({
		options: OPTIONS,
		matched: product === "Heritage G" ? ["Frac11"] : [],
	}));
});

afterEach(() => {
	vi.useRealTimers();
	vi.clearAllMocks();
});

/** Let the debounce elapse and the lookup settle. */
async function pause(ms = 300) {
	await act(async () => {
		await vi.advanceTimersByTimeAsync(ms);
	});
}

function picker(productName: string, value: string[] = [], recorded = false) {
	const onChange = vi.fn();
	const view = render(
		<FracClassPicker
			productName={productName}
			value={value}
			onChange={onChange}
			recorded={recorded}
		/>,
	);
	const rename = (name: string, next: string[] = value) =>
		view.rerender(
			<FracClassPicker
				productName={name}
				value={next}
				onChange={onChange}
				recorded={recorded}
			/>,
		);
	return { onChange, rename };
}

describe("FRAC class picker", () => {
	// @spec APP-FRAC-002
	it("shows a checkbox per class with its example products as the tooltip", async () => {
		picker("");
		await pause();
		const dmi = screen.getByLabelText("FRAC 3 — DMIs");
		expect(dmi).toHaveAttribute("type", "checkbox");
		expect(dmi.closest("label")).toHaveAttribute(
			"title",
			"Banner Maxx, Bayleton",
		);
		expect(screen.getByLabelText("FRAC 11 — Strobilurins")).toBeInTheDocument();
	});

	// @spec APP-FRAC-003
	it("looks the name up after a 300 ms pause and ticks what it matches", async () => {
		const { onChange, rename } = picker("");
		await pause();
		onChange.mockClear();
		rename("Heritage G");
		await pause(299);
		expect(api.getFracClasses).not.toHaveBeenCalledWith("Heritage G");
		await pause(1);
		expect(api.getFracClasses).toHaveBeenLastCalledWith("Heritage G");
		expect(onChange).toHaveBeenLastCalledWith(["Frac11"]);
	});

	// @spec APP-FRAC-003
	it("never overwrites recorded classes, a picked product's, or the owner's ticks", async () => {
		const recorded = picker("Heritage G", ["Frac3"], true);
		await pause();
		expect(recorded.onChange).not.toHaveBeenCalled();

		const owner = picker("");
		await pause();
		fireEvent.click(
			screen.getAllByLabelText("FRAC 3 — DMIs").at(-1) as HTMLElement,
		);
		expect(owner.onChange).toHaveBeenLastCalledWith(["Frac3"]);
		owner.onChange.mockClear();
		owner.rename("Heritage G", ["Frac3"]);
		await pause();
		expect(owner.onChange).not.toHaveBeenCalled();
	});

	// @spec APP-FRAC-004
	it("warns when a named product matches no class and none is ticked", async () => {
		const { rename } = picker("Mystery Spray");
		await pause();
		expect(screen.getByRole("alert")).toHaveTextContent(
			"TurfOps doesn't recognize this product",
		);
		expect(screen.getByRole("alert")).toHaveTextContent(
			"can't count toward disease protection or fungicide rotation",
		);
		rename("Mystery Spray", ["Frac3"]);
		expect(screen.queryByRole("alert")).not.toBeInTheDocument();
		rename("", []);
		await pause();
		expect(screen.queryByRole("alert")).not.toBeInTheDocument();
		rename("Heritage G", []);
		await pause();
		expect(screen.queryByRole("alert")).not.toBeInTheDocument();
	});
});
