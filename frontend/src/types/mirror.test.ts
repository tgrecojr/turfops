// The hand-written mirror of the Rust models, checked against the Rust source (runtime) and
// against its own declared shapes (compile time: `tsc -b` type-checks this file).
import { describe, expect, expectTypeOf, it } from "vitest";
import applicationRs from "../../../backend/src/models/application.rs?raw";
import lawnProfileRs from "../../../backend/src/models/lawn_profile.rs?raw";
import plantRs from "../../../backend/src/models/plant.rs?raw";
import recommendationRs from "../../../backend/src/models/recommendation.rs?raw";
import type {
	Application,
	ApplicationType,
	GrassType,
	IrrigationType,
	LawnProfile,
	PlantType,
	Severity,
	SoilType,
} from ".";
import {
	APPLICATION_TYPE_COLORS,
	APPLICATION_TYPE_LABELS,
	GRASS_TYPE_LABELS,
	PLANT_TYPE_LABELS,
	SEVERITY_COLORS,
	SEVERITY_SYMBOLS,
} from ".";

/** The variant names of `pub enum <name>` in a Rust source file. */
function rustVariants(source: string, name: string): string[] {
	const start = source.indexOf(`pub enum ${name} {`);
	if (start === -1) throw new Error(`no enum ${name}`);
	const body = source.slice(
		source.indexOf("{", start) + 1,
		source.indexOf("}", start),
	);
	return body
		.replace(/\/\/.*$/gm, "")
		.split(",")
		.map((v) => v.trim())
		.filter(Boolean);
}

describe("type mirror", () => {
	// @spec UI-TYPE-002
	it("names each enum's values exactly as the Rust variants", () => {
		const cases: [string, string, Record<string, string>][] = [
			[lawnProfileRs, "GrassType", GRASS_TYPE_LABELS],
			[applicationRs, "ApplicationType", APPLICATION_TYPE_LABELS],
			[recommendationRs, "Severity", SEVERITY_COLORS],
			[plantRs, "PlantType", PLANT_TYPE_LABELS],
		];
		for (const [source, name, map] of cases) {
			expect(Object.keys(map).sort(), name).toEqual(
				rustVariants(source, name).sort(),
			);
		}
		expectTypeOf<SoilType>().toEqualTypeOf<
			"Clay" | "Loam" | "Sandy" | "SiltLoam" | "ClayLoam" | "SandyLoam"
		>();
		expect(rustVariants(lawnProfileRs, "SoilType").sort()).toEqual(
			["Clay", "Loam", "Sandy", "SiltLoam", "ClayLoam", "SandyLoam"].sort(),
		);
		expectTypeOf<IrrigationType>().toEqualTypeOf<
			"InGround" | "Hose" | "None"
		>();
		expect(rustVariants(lawnProfileRs, "IrrigationType")).toEqual([
			"InGround",
			"Hose",
			"None",
		]);
	});

	// @spec UI-TYPE-002
	it("carries Option as null, omitted fields as optional and dates as strings", () => {
		// Option<String> / Option<SoilType>, always sent.
		expectTypeOf<Application["product_name"]>().toEqualTypeOf<string | null>();
		expectTypeOf<LawnProfile["soil_type"]>().toEqualTypeOf<SoilType | null>();
		// `skip_serializing_if = "Option::is_none"`: the field may be absent.
		expectTypeOf<Pick<Application, "product_id">>().toEqualTypeOf<{
			product_id?: number | null;
		}>();
		expect(applicationRs).toMatch(
			/skip_serializing_if = "Option::is_none"\)\]\n\s*pub product_id: Option<i64>/,
		);
		// NaiveDate and DateTime<Utc> arrive as ISO strings.
		expectTypeOf<Application["application_date"]>().toEqualTypeOf<string>();
		expectTypeOf<Application["created_at"]>().toEqualTypeOf<string>();
		expect(applicationRs).toMatch(/pub application_date: NaiveDate,/);
	});

	// @spec UI-TYPE-003
	it("keeps each enum's display metadata in an exhaustive map", () => {
		expectTypeOf(APPLICATION_TYPE_LABELS).toEqualTypeOf<
			Record<ApplicationType, string>
		>();
		expectTypeOf(APPLICATION_TYPE_COLORS).toEqualTypeOf<
			Record<ApplicationType, string>
		>();
		expectTypeOf(GRASS_TYPE_LABELS).toEqualTypeOf<Record<GrassType, string>>();
		expectTypeOf(PLANT_TYPE_LABELS).toEqualTypeOf<Record<PlantType, string>>();
		expectTypeOf(SEVERITY_COLORS).toEqualTypeOf<Record<Severity, string>>();
		expectTypeOf(SEVERITY_SYMBOLS).toEqualTypeOf<Record<Severity, string>>();
		// @ts-expect-error a map that leaves out a variant does not compile
		const missing: Record<Severity, string> = { Info: "", Advisory: "" };
		expect(Object.keys(missing)).toHaveLength(2);
		for (const map of [
			APPLICATION_TYPE_LABELS,
			APPLICATION_TYPE_COLORS,
			GRASS_TYPE_LABELS,
			PLANT_TYPE_LABELS,
			SEVERITY_COLORS,
			SEVERITY_SYMBOLS,
		]) {
			expect(Object.values(map).every((v) => v !== "")).toBe(true);
		}
	});
});
