// Response fixtures for the Environmental page and soil-outlook component tests.

import type {
	EnvironmentalSummary,
	HistoricalData,
	SoilTempForecast,
	TimeSeriesPoint,
} from "../types";

export function summary(
	overrides: Partial<EnvironmentalSummary> = {},
): EnvironmentalSummary {
	return {
		current: {
			timestamp: "2026-10-10T14:00:00Z",
			source: "SoilData",
			soil_temp_5_f: 61.2,
			soil_temp_10_f: 62.4,
			soil_temp_20_f: 63.6,
			soil_temp_50_f: 64.8,
			soil_temp_100_f: 66.1,
			soil_moisture_5: 0.21,
			soil_moisture_10: 0.25,
			soil_moisture_20: 0.27,
			soil_moisture_50: 0.3,
			soil_moisture_100: 0.33,
			ambient_temp_f: 71.5,
			humidity_percent: 55.5,
			precipitation_mm: 0,
		},
		soil_temp_7day_avg_f: 60.3,
		ambient_temp_7day_avg_f: 65.7,
		humidity_7day_avg: 70.2,
		precipitation_7day_total_mm: 25.4,
		soil_temp_trend: "Rising",
		last_updated: "2026-10-11T12:00:00Z",
		forecast: {
			fetched_at: "2026-10-11T12:00:00Z",
			location: {
				city: "Avondale",
				country: "US",
				latitude: 39.8,
				longitude: -75.8,
			},
			hourly: [],
			daily_summary: [
				{
					date: "2026-10-12",
					high_temp_f: 68.4,
					low_temp_f: 47.6,
					avg_humidity: 64.4,
					total_precipitation_mm: 25.4,
					max_precipitation_prob: 0.4,
					dominant_condition: "Rain",
					avg_wind_speed_mph: 5,
					max_wind_gust_mph: null,
					hours_covered: 24,
				},
				{
					date: "2026-10-13",
					high_temp_f: 72.2,
					low_temp_f: 50.1,
					avg_humidity: 58,
					total_precipitation_mm: 0,
					max_precipitation_prob: 0,
					dominant_condition: "Clear",
					avg_wind_speed_mph: 4,
					max_wind_gust_mph: null,
					hours_covered: 24,
				},
			],
		},
		gdd_base50_ytd: 3100,
		...overrides,
	};
}

export function points(values: number[], start = "2026-10-10T00:00:00Z") {
	const t0 = new Date(start).getTime();
	return values.map(
		(value, i): TimeSeriesPoint => ({
			timestamp: new Date(t0 + i * 3_600_000).toISOString(),
			value,
		}),
	);
}

export function historical(
	range = "7d",
	overrides: Partial<HistoricalData> = {},
): HistoricalData {
	return {
		range,
		soil_temp_10_f: points([60, 61]),
		ambient_temp_f: points([70, 71]),
		humidity_percent: points([50, 55]),
		soil_moisture_10: points([0.2, 0.21]),
		precipitation_mm: points([0, 25.4]),
		gdd_accumulation: points([3000, 3010]),
		...overrides,
	};
}

export const EMPTY_HISTORY: HistoricalData = {
	range: "7d",
	soil_temp_10_f: [],
	ambient_temp_f: [],
	humidity_percent: [],
	soil_moisture_10: [],
	precipitation_mm: [],
	gdd_accumulation: [],
};

export function soilForecast(
	overrides: Partial<SoilTempForecast> = {},
): SoilTempForecast {
	return {
		predictions: [
			{
				date: "2026-10-12",
				predicted_soil_temp_f: 61.5,
				confidence: "High",
				air_temp_used_f: 58,
				source_description: "Forecast air temp from 2026-10-12",
			},
			{
				date: "2026-10-13",
				predicted_soil_temp_f: 58.2,
				confidence: "Medium",
				air_temp_used_f: 52,
				source_description: "Forecast air temp from 2026-10-13",
			},
		],
		threshold_crossings: [
			{
				threshold_name: "Active Growth Peak",
				threshold_temp_f: 65,
				estimated_crossing_date: "2026-10-14",
				days_until_crossing: 3,
				beyond_forecast: false,
				confidence: "Medium",
				direction: "Falling",
			},
		],
		model_info: {
			r_squared: 0.8234,
			lag_days: 2,
			training_window_days: 30,
			quality: "Good",
		},
		generated_at: "2026-10-11T12:00:00Z",
		...overrides,
	};
}
