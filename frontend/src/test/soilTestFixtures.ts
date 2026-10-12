// Soil tests and advice for component tests.
import type {
	NpkRecommendation,
	PhRecommendation,
	SoilTest,
	SoilTestSummary,
} from "../types";

export function soilTest(overrides: Partial<SoilTest> = {}): SoilTest {
	return {
		id: 1,
		lawn_profile_id: 1,
		test_date: "2026-03-23",
		lab_name: "Waypoint",
		ph: 6.0,
		buffer_ph: null,
		phosphorus_ppm: 10,
		potassium_ppm: 80,
		calcium_ppm: 300,
		magnesium_ppm: 44,
		sulfur_ppm: null,
		iron_ppm: null,
		manganese_ppm: null,
		zinc_ppm: null,
		boron_ppm: null,
		copper_ppm: null,
		organic_matter_pct: 3.2,
		cec: null,
		notes: null,
		created_at: "2026-03-25T12:00:00Z",
		...overrides,
	};
}

export const phAdvice: PhRecommendation = {
	current_ph: 6.0,
	target_ph: 6.5,
	amendment: "Lime (calcitic or dolomitic)",
	rate_lbs_per_1000sqft: 25,
	explanation:
		"Your soil pH is 6.0, target is 6.5. Apply lime at 25 lbs/1000 sqft.",
};

export const npkAdvice: NpkRecommendation = {
	phosphorus_level: "Low",
	potassium_level: "Low",
	recommended_ratio: "3-1-2",
	nitrogen_rate_lbs_per_1000sqft: 0.75,
	phosphorus_rate_lbs_per_1000sqft: 0.25,
	potassium_rate_lbs_per_1000sqft: 0.5,
	product_rate_lbs_per_1000sqft: 6.3,
	example_product_ratio: "12-4-8",
	remaining_n_budget_lbs_per_1000sqft: 3.5,
	explanation:
		"Recommend 0.75 lbs N per 1000 sqft with a 3-1-2 ratio fertilizer.",
};

export function summary(
	overrides: Partial<SoilTestSummary> = {},
): SoilTestSummary {
	return {
		soil_test: soilTest(),
		ph_recommendation: phAdvice,
		npk_recommendation: npkAdvice,
		micronutrient_recommendations: [
			{
				nutrient: "Calcium",
				current_ppm: 300,
				threshold_ppm: 500,
				level: "Low",
				suggestion: "Apply gypsum (calcium sulfate) at 20-40 lbs/1000 sqft",
			},
		],
		...overrides,
	};
}
