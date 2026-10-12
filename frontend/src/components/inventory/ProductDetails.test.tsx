// @vitest-environment jsdom
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import ProductDetails from "./ProductDetails";
import { makeProduct, makeProfile } from "./testFixtures";

const ASSISTANT =
	"— suggested by the assistant; verify against your label. Pre-fills the application form only.";
const OWNER = "— from your label. Pre-fills the application form only.";

// @spec INV-RATE-002
describe("ProductDetails label rate", () => {
	it("flags a rate that is still the assistant's suggestion", () => {
		render(
			<ProductDetails
				product={makeProduct({
					profile: makeProfile(),
					label_rate_per_1000sqft: 2,
					label_rate_unit: "Lb",
				})}
			/>,
		);
		expect(screen.getByText(/2 lb\s+per 1,000 sq ft/)).toBeVisible();
		expect(screen.getByText(ASSISTANT)).toBeVisible();
	});

	it("credits the owner's label once the rate differs", () => {
		render(
			<ProductDetails
				product={makeProduct({
					profile: makeProfile(),
					label_rate_per_1000sqft: 3.5,
					label_rate_unit: "FlOz",
				})}
			/>,
		);
		expect(screen.getByText(/3.5 fl oz\s+per 1,000 sq ft/)).toBeVisible();
		expect(screen.getByText(OWNER)).toBeVisible();
		expect(screen.queryByText(ASSISTANT)).toBeNull();
	});

	it("shows no rate line when none is recorded", () => {
		render(<ProductDetails product={makeProduct()} />);
		expect(screen.queryByText("Label rate")).toBeNull();
	});
});

// @spec INV-UI-005
describe("ProductDetails confidence", () => {
	it("warns when the profile's confidence is not High", () => {
		render(
			<ProductDetails
				product={makeProduct({
					profile: makeProfile({ confidence: "Medium" }),
				})}
			/>,
		);
		expect(
			screen.getByText(/Medium identification confidence — verify/),
		).toBeVisible();
	});

	it("does not warn at High confidence", () => {
		render(
			<ProductDetails product={makeProduct({ profile: makeProfile() })} />,
		);
		expect(screen.queryByText(/identification confidence — verify/)).toBeNull();
	});
});
