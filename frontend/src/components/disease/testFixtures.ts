// Test data shaped like `GET /api/v1/disease-risk`.
import type {
	DailyRisk,
	DiseaseManagement,
	DiseaseRisk,
	DiseaseRiskResponse,
	FungicideOption,
} from "../../types/disease";

export const TODAY = "2026-09-20";

export function option(overrides: Partial<FungicideOption>): FungicideOption {
	return {
		frac_class: "Frac11",
		class_label: "FRAC 11 (Strobilurins)",
		examples: ["azoxystrobin", "Heritage"],
		efficacy: "Excellent",
		note: null,
		last_used: false,
		recommended: false,
		restricted: false,
		on_hand: [],
		...overrides,
	};
}

export function management(
	overrides: Partial<DiseaseManagement> = {},
): DiseaseManagement {
	return {
		action: "ApplyPreventative",
		headline: "Apply a preventative: FRAC 7 (SDHI), e.g. fluxapyroxad.",
		cultural: ["Water only between 4 and 7 AM.", "Mow at 3.5–4 in."],
		preventative: {
			when: "Before symptoms.",
			interval: "14–28 days",
			guidance: ["Use the label's preventative rate."],
			options: [
				option({ last_used: true }),
				option({
					frac_class: "Frac7",
					class_label: "FRAC 7 (SDHI)",
					examples: ["fluxapyroxad", "Xzemplar"],
					recommended: true,
					on_hand: [
						{ product_id: 4, name: "Xzemplar", stock_status: "InStock" },
					],
				}),
			],
		},
		curative: {
			when: "Once you can see active symptoms.",
			interval: "14 days",
			guidance: ["Use the label's curative (higher) rate."],
			options: [
				option({
					frac_class: "Frac3",
					class_label: "FRAC 3 (DMIs)",
					examples: ["propiconazole"],
					efficacy: "Good",
					recommended: true,
				}),
			],
		},
		protection: null,
		notes: ["Rates are product-specific — follow the product label."],
		...overrides,
	};
}

export function day(
	date: string,
	value: number,
	overrides: Partial<DailyRisk> = {},
): DailyRisk {
	return {
		date,
		value,
		tier:
			value >= 6
				? "Severe"
				: value >= 5
					? "High"
					: value >= 0
						? "Moderate"
						: "Low",
		is_forecast: date > TODAY,
		partial: false,
		...overrides,
	};
}

export function risk(overrides: Partial<DiseaseRisk> = {}): DiseaseRisk {
	return {
		disease: "BrownPatch",
		slug: "brown-patch",
		name: "Brown Patch",
		pathogen: "Rhizoctonia solani",
		tier: "High",
		tier_note: null,
		score: 5.4,
		score_label: "E-index 5.4",
		as_of: TODAY,
		scale: {
			min: -5,
			max: 10,
			unit: "E-index",
			moderate_at: 0,
			high_at: 5,
			severe_at: 6,
		},
		daily: [
			day("2026-09-19", 4.2),
			day(TODAY, 5.4),
			day("2026-09-21", 6.3),
			day("2026-09-22", 3.1, { partial: true }),
		],
		factors: [
			{
				label: "Overnight low",
				value: "68°F",
				status: "Favorable",
				note: "Warm night",
			},
			{
				label: "Mean relative humidity",
				value: "80%",
				status: "Marginal",
				note: "Elevated",
			},
			{
				label: "E-index",
				value: "-1.00",
				status: "Unfavorable",
				note: "Below 0",
			},
		],
		summary: "The brown patch E-index is 5.4.",
		methodology: {
			model_name: "Fidanza-Dernoeden E-index",
			citation: "Fidanza et al. (1996)",
			validated: true,
			summary: "A field-validated brown patch warning model.",
			steps: ["Take Tmin and RH.", "E = −21.5 + 0.15·RH"],
		},
		management: management(),
		...overrides,
	};
}

export function response(
	overrides: Partial<DiseaseRiskResponse> = {},
): DiseaseRiskResponse {
	return {
		generated_at: "2026-09-20T14:00:00Z",
		today: TODAY,
		station: "NOAA USCRN station 3761",
		diseases: [
			risk(),
			risk({
				disease: "DollarSpot",
				slug: "dollar-spot",
				name: "Dollar Spot",
				tier: "Low",
				score: 4,
				score_label: "4% probability",
				scale: {
					min: 0,
					max: 100,
					unit: "%",
					moderate_at: 10,
					high_at: 20,
					severe_at: 40,
				},
				methodology: {
					model_name: "Smith-Kerns dollar spot model",
					citation: "Smith et al. (2018)",
					validated: true,
					summary: "Logistic regression.",
					steps: ["logit = …"],
				},
			}),
		],
		data_notes: ["Forecast unavailable — no outlook days are shown."],
		...overrides,
	};
}
