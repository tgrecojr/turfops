// @vitest-environment jsdom
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { StockStatus } from "../../types/inventory";
import { nextStockStatus } from "../../types/inventory";
import StockBadge from "./StockBadge";

// @spec INV-UI-010
describe("StockBadge", () => {
	it.each([
		["InStock", "✓", "In stock", "#0ca30c"],
		["Low", "△", "Running low", "#fab219"],
		["Out", "✕", "Out", "#d03b3b"],
	] as [StockStatus, string, string, string][])(
		"shows %s as symbol and label over its status color",
		(status, symbol, label, color) => {
			render(<StockBadge status={status} />);
			const pill = screen.getByText(label, { exact: false });
			expect(pill).toHaveTextContent(`${symbol} ${label}`);
			expect(pill).toHaveStyle({ borderColor: color });
			expect(pill).toHaveStyle({ color: "#2d3748" });
		},
	);
});

// @spec INV-UI-006
describe("stock pill cycle", () => {
	it("goes In stock → Low → Out → In stock", () => {
		expect(nextStockStatus("InStock")).toBe("Low");
		expect(nextStockStatus("Low")).toBe("Out");
		expect(nextStockStatus("Out")).toBe("InStock");
	});

	it("asks the page to cycle when clicked", async () => {
		const onCycle = vi.fn();
		render(<StockBadge status="Low" onCycle={onCycle} />);
		await userEvent.click(screen.getByRole("button", { name: /running low/i }));
		expect(onCycle).toHaveBeenCalledOnce();
	});
});
