import type { ShelfProduct } from "./inventory";
// Mirrors backend/src/models/disease.rs

export type Disease =
	| "BrownPatch"
	| "DollarSpot"
	| "PythiumBlight"
	| "GrayLeafSpot"
	| "RedThread";

export type RiskTier = "Low" | "Moderate" | "High" | "Severe";

export type FactorStatus = "Unfavorable" | "Marginal" | "Favorable";

export interface DailyRisk {
	date: string;
	value: number;
	tier: RiskTier;
	is_forecast: boolean;
	partial: boolean;
}

export interface RiskFactor {
	label: string;
	value: string;
	status: FactorStatus;
	note: string;
}

export interface RiskScale {
	min: number;
	max: number;
	unit: string;
	moderate_at: number;
	high_at: number;
	severe_at: number;
}

export interface Methodology {
	model_name: string;
	citation: string;
	validated: boolean;
	summary: string;
	steps: string[];
}

export type ManagementAction =
	| "NoAction"
	| "Monitor"
	| "Cultural"
	| "ApplyPreventative"
	| "Protected";

export type Efficacy = "Fair" | "Good" | "Excellent";

export interface FungicideOption {
	frac_class: string;
	class_label: string;
	examples: string[];
	efficacy: Efficacy;
	note: string | null;
	last_used: boolean;
	recommended: boolean;
	/** Never picked (e.g. not labeled for residential lawns). */
	restricted: boolean;
	/** Products on the shelf in this class. */
	on_hand: ShelfProduct[];
}

export interface FungicideProgram {
	when: string;
	interval: string;
	guidance: string[];
	options: FungicideOption[];
}

export interface ProtectionStatus {
	product: string;
	class_label: string;
	applied_on: string;
	protected_through: string;
	days_remaining: number;
}

export interface DiseaseManagement {
	action: ManagementAction;
	headline: string;
	cultural: string[];
	preventative: FungicideProgram;
	curative: FungicideProgram;
	protection: ProtectionStatus | null;
	notes: string[];
}

export interface DiseaseRisk {
	disease: Disease;
	slug: string;
	name: string;
	pathogen: string;
	tier: RiskTier;
	tier_note: string | null;
	score: number;
	score_label: string;
	as_of: string;
	scale: RiskScale;
	daily: DailyRisk[];
	factors: RiskFactor[];
	summary: string;
	methodology: Methodology;
	management: DiseaseManagement;
}

export interface DiseaseRiskResponse {
	generated_at: string;
	today: string;
	station: string;
	diseases: DiseaseRisk[];
	data_notes: string[];
}

export const RISK_TIERS: RiskTier[] = ["Low", "Moderate", "High", "Severe"];

// Status palette (good / warning / serious / critical). Tier is never carried by
// color alone — always pair with the symbol + label.
export const RISK_TIER_COLORS: Record<RiskTier, string> = {
	Low: "#0ca30c",
	Moderate: "#fab219",
	High: "#ec835a",
	Severe: "#d03b3b",
};

export const RISK_TIER_SYMBOLS: Record<RiskTier, string> = {
	Low: "✓",
	Moderate: "△",
	High: "⚠",
	Severe: "!",
};

// A factor that is *favorable to the disease* is bad news for the lawn.
export const FACTOR_STATUS_COLORS: Record<FactorStatus, string> = {
	Unfavorable: "#0ca30c",
	Marginal: "#fab219",
	Favorable: "#d03b3b",
};

export const FACTOR_STATUS_SYMBOLS: Record<FactorStatus, string> = {
	Unfavorable: "✓",
	Marginal: "△",
	Favorable: "⚠",
};

export const MANAGEMENT_ACTION_LABELS: Record<ManagementAction, string> = {
	NoAction: "No action",
	Monitor: "Monitor",
	Cultural: "Cultural fix",
	ApplyPreventative: "Apply preventative",
	Protected: "Protected",
};

/** Format an ISO `YYYY-MM-DD` date as `M/D` without timezone shifts. */
export function formatShortDate(iso: string): string {
	const [, month, day] = iso.split("-");
	return `${Number(month)}/${Number(day)}`;
}
