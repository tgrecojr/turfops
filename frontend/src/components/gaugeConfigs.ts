export interface GaugeConfig {
	label: string;
	unit: string;
	min: number;
	max: number;
	thresholds: { warn: number; critical: number };
	/** Low-side warnings (dry soil), when the gauge has any. */
	dry?: { warn: number; critical: number };
}

export type GaugeLevel = "ok" | "warning" | "critical";

/** The gauge's state for `value`, with the word that says so (never color alone). */
// @spec WX-VIEW-PAGE-010, WX-VIEW-PAGE-011
export function gaugeLevel(
	config: Pick<GaugeConfig, "thresholds" | "dry">,
	value: number | null,
): { level: GaugeLevel; label: string | null } {
	if (value === null) return { level: "ok", label: null };
	const { thresholds, dry } = config;
	if (value >= thresholds.critical)
		return { level: "critical", label: "Very high" };
	if (value >= thresholds.warn) return { level: "warning", label: "High" };
	if (dry && value < dry.critical)
		return { level: "critical", label: "Very dry" };
	if (dry && value < dry.warn) return { level: "warning", label: "Dry" };
	return { level: "ok", label: null };
}

export const SOIL_TEMP_GAUGE: GaugeConfig = {
	label: "Soil Temp (10cm)",
	unit: "\u00B0F",
	min: 30,
	max: 100,
	thresholds: { warn: 75, critical: 85 },
};

export const AMBIENT_TEMP_GAUGE: GaugeConfig = {
	label: "Ambient Temp",
	unit: "\u00B0F",
	min: 0,
	max: 110,
	thresholds: { warn: 85, critical: 95 },
};

export const HUMIDITY_GAUGE: GaugeConfig = {
	label: "Humidity",
	unit: "%",
	min: 0,
	max: 100,
	thresholds: { warn: 80, critical: 90 },
};

export const SOIL_MOISTURE_GAUGE: GaugeConfig = {
	label: "Soil Moisture (10cm)",
	unit: "%",
	min: 0,
	max: 50,
	thresholds: { warn: 40, critical: 45 },
	// Volumetric %, so 15 / 10 are moisture 0.15 / 0.10.
	dry: { warn: 15, critical: 10 },
};
