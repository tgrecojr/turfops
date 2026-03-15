// Centralized agronomic thresholds for TTTF Zone 7a.
//
// All numeric thresholds used across the 18 rule files are defined here
// for easy reference, maintenance, and future configurability.

// =============================================================================
// Temperature thresholds (°F)
// =============================================================================

/// Ambient temperature above which cool-season grasses experience heat stress.
/// Fertilizer applications should be avoided above this threshold.
pub const HEAT_STRESS_TEMP_F: f64 = 85.0;

/// Temperature above which heat stress severity escalates to Warning.
pub const HEAT_STRESS_WARNING_TEMP_F: f64 = 90.0;

/// Temperature above which heat stress severity escalates to Critical.
pub const HEAT_STRESS_CRITICAL_TEMP_F: f64 = 95.0;

/// Cool-season grass ideal growth range — low end (°F).
pub const COOL_SEASON_IDEAL_LOW_F: f64 = 60.0;

/// Cool-season grass ideal growth range — high end (°F).
pub const COOL_SEASON_IDEAL_HIGH_F: f64 = 75.0;

// -- Soil temperature windows --

/// Pre-emergent application window — lower bound (°F, 7-day avg).
pub const PRE_EMERGENT_SOIL_LOW_F: f64 = 50.0;

/// Pre-emergent application window — upper bound (°F, 7-day avg).
pub const PRE_EMERGENT_SOIL_HIGH_F: f64 = 60.0;

/// Crabgrass germination risk threshold — urgency increases above this.
pub const PRE_EMERGENT_URGENCY_SOIL_F: f64 = 55.0;

/// Pre-emergent late window — efficacy drops significantly above this.
pub const PRE_EMERGENT_LATE_SOIL_F: f64 = 70.0;

/// Grub control application window — lower bound (°F, 7-day avg).
pub const GRUB_CONTROL_SOIL_LOW_F: f64 = 60.0;

/// Grub control application window — upper bound (°F, 7-day avg).
pub const GRUB_CONTROL_SOIL_HIGH_F: f64 = 75.0;

/// Core aeration window — lower bound (°F, 7-day avg).
pub const AERATION_SOIL_LOW_F: f64 = 50.0;

/// Core aeration window — upper bound (°F, 7-day avg).
pub const AERATION_SOIL_HIGH_F: f64 = 65.0;

/// Fall overseeding optimal soil temp — lower bound (°F).
pub const OVERSEED_SOIL_LOW_F: f64 = 50.0;

/// Fall overseeding optimal soil temp — upper bound (°F).
pub const OVERSEED_SOIL_HIGH_F: f64 = 65.0;

/// Fall overseeding peak germination — lower bound (°F).
pub const OVERSEED_PEAK_LOW_F: f64 = 55.0;

/// Fall overseeding peak germination — upper bound (°F).
pub const OVERSEED_PEAK_HIGH_F: f64 = 62.0;

/// Fall overseeding — soil too warm, wait for cooler (°F).
pub const OVERSEED_WARM_LIMIT_F: f64 = 75.0;

/// Fall fertilization — soil temp OK range lower bound (°F).
pub const FALL_FERT_SOIL_LOW_F: f64 = 45.0;

/// Fall fertilization — soil temp OK range upper bound (°F).
pub const FALL_FERT_SOIL_HIGH_F: f64 = 65.0;

/// Winterizer minimum soil temp (°F).
pub const WINTERIZER_MIN_SOIL_F: f64 = 40.0;

/// Spring nitrogen — minimum soil temp to begin fertilizing (°F).
pub const SPRING_N_MIN_SOIL_F: f64 = 55.0;

/// Spring nitrogen — approaching threshold (°F, 7-day avg).
pub const SPRING_N_APPROACHING_SOIL_F: f64 = 50.0;

/// Spring nitrogen — upper ready range (°F).
pub const SPRING_N_READY_HIGH_F: f64 = 65.0;

/// Spring broadleaf herbicide — soil temp lower bound (°F).
pub const SPRING_HERBICIDE_SOIL_LOW_F: f64 = 45.0;

/// Spring broadleaf herbicide — soil temp upper bound (°F).
pub const SPRING_HERBICIDE_SOIL_HIGH_F: f64 = 55.0;

/// Fall broadleaf herbicide — soil temp lower bound (°F).
pub const FALL_HERBICIDE_SOIL_LOW_F: f64 = 50.0;

/// Fall broadleaf herbicide — soil temp upper bound (°F).
pub const FALL_HERBICIDE_SOIL_HIGH_F: f64 = 65.0;

// -- Disease temperature thresholds --

/// Brown patch onset — night temp (°F). NC State Extension.
pub const BROWN_PATCH_NIGHT_ONSET_F: f64 = 60.0;

/// Brown patch elevated risk — night temp (°F).
pub const BROWN_PATCH_NIGHT_ELEVATED_F: f64 = 65.0;

/// Brown patch severe — night temp (°F).
pub const BROWN_PATCH_NIGHT_SEVERE_F: f64 = 70.0;

/// Brown patch severe — day temp (°F).
pub const BROWN_PATCH_DAY_SEVERE_F: f64 = 90.0;

/// Dollar spot onset — night temp (°F). NC State Extension.
pub const DOLLAR_SPOT_NIGHT_ONSET_F: f64 = 50.0;

/// Dollar spot distinguishing threshold — below this = dollar spot, above = brown patch.
pub const DOLLAR_SPOT_NIGHT_UPPER_F: f64 = 68.0;

/// Disease pressure warm day range — lower bound (°F).
pub const DISEASE_WARM_DAY_LOW_F: f64 = 75.0;

/// Disease pressure warm day range — upper bound (°F).
pub const DISEASE_WARM_DAY_HIGH_F: f64 = 90.0;

/// Pythium blight — minimum night temp (°F).
pub const PYTHIUM_NIGHT_MIN_F: f64 = 65.0;

/// Pythium blight — minimum day temp (°F).
pub const PYTHIUM_DAY_MIN_F: f64 = 85.0;

/// Gray leaf spot — ambient temp lower bound (°F).
pub const GRAY_LEAF_SPOT_TEMP_LOW_F: f64 = 70.0;

/// Gray leaf spot — ambient temp upper bound (°F).
pub const GRAY_LEAF_SPOT_TEMP_HIGH_F: f64 = 95.0;

/// Red thread — ambient temp lower bound (°F).
pub const RED_THREAD_TEMP_LOW_F: f64 = 40.0;

/// Red thread — ambient temp upper bound (°F).
pub const RED_THREAD_TEMP_HIGH_F: f64 = 80.0;

// -- Mowing height temperature breaks --

/// Mowing height — heat stress break (°F, 7-day avg ambient).
pub const MOWING_HEAT_STRESS_TEMP_F: f64 = 85.0;

/// Mowing height — summer break (°F, 7-day avg ambient).
pub const MOWING_SUMMER_TEMP_F: f64 = 75.0;

/// Mowing height — spring/fall lower bound (°F, 7-day avg ambient).
pub const MOWING_ACTIVE_GROWTH_TEMP_F: f64 = 50.0;

// -- Application window temperature --

/// Application window — max acceptable high temp (°F).
pub const APP_WINDOW_MAX_HIGH_F: f64 = 85.0;

/// Application window — ideal temp lower bound (°F).
pub const APP_WINDOW_IDEAL_LOW_F: f64 = 55.0;

/// Application window — ideal temp upper bound (°F).
pub const APP_WINDOW_IDEAL_HIGH_F: f64 = 75.0;

/// Application window — minimum avg temp (°F).
pub const APP_WINDOW_MIN_AVG_F: f64 = 50.0;

/// Forecast average high above which overseeding seedlings are stressed.
pub const OVERSEED_FORECAST_HOT_F: f64 = 85.0;

// =============================================================================
// Soil moisture thresholds (volumetric fraction, 0.0–1.0)
// =============================================================================

/// Below this soil moisture, drought stress occurs and irrigation is critical.
pub const SOIL_MOISTURE_DROUGHT: f64 = 0.10;

/// Below this soil moisture, severe drought — fertilizer severity escalates to Critical.
pub const SOIL_MOISTURE_SEVERE_DROUGHT: f64 = 0.05;

/// Irrigation warning threshold — moisture below this needs attention soon.
pub const SOIL_MOISTURE_IRRIGATION_WARNING: f64 = 0.15;

/// Above this moisture, irrigation not needed.
pub const SOIL_MOISTURE_ADEQUATE: f64 = 0.20;

/// Above this soil moisture, soil is saturated — avoid fertilizer (risk of leaching).
pub const SOIL_MOISTURE_SATURATED: f64 = 0.40;

// =============================================================================
// Humidity thresholds (%)
// =============================================================================

/// Humidity above which disease risk begins (general fungal diseases).
pub const HUMIDITY_DISEASE_RISK: f64 = 80.0;

/// Humidity above which disease risk is elevated (gray leaf spot, etc.).
pub const HUMIDITY_HIGH_DISEASE: f64 = 85.0;

/// Humidity above which disease risk is severe.
pub const HUMIDITY_SEVERE_DISEASE: f64 = 90.0;

/// Red thread / sustained humidity threshold.
pub const HUMIDITY_RED_THREAD: f64 = 75.0;

/// Application window — low humidity bonus threshold.
pub const HUMIDITY_APP_WINDOW_LOW: f64 = 70.0;

/// Application window — max acceptable humidity.
pub const HUMIDITY_APP_WINDOW_MAX: f64 = 85.0;

// =============================================================================
// Precipitation thresholds
// =============================================================================

/// Minimum meaningful precipitation (mm) — approximately 0.1 inch.
pub const PRECIP_TRACE_MM: f64 = 2.5;

/// Heavy 7-day precipitation (mm) — approximately 1 inch.
pub const PRECIP_HEAVY_7DAY_MM: f64 = 25.0;

/// Minimum rain amount for forecast rain check (inches).
pub const PRECIP_FORECAST_MIN_INCHES: f64 = 0.1;

/// Precipitation probability threshold for application window / rain delay.
pub const PRECIP_PROB_LIKELY: f64 = 0.5;

/// Precipitation probability threshold for Pythium/thunderstorm risk.
pub const PRECIP_PROB_THUNDERSTORM: f64 = 0.6;

/// Rain delay — critical probability threshold.
pub const RAIN_DELAY_CRITICAL_PROB: f64 = 0.7;

/// Rain delay — advisory probability threshold.
pub const RAIN_DELAY_ADVISORY_PROB: f64 = 0.3;

/// Rain delay — advisory expected amount (mm, ~0.2 inch).
pub const RAIN_DELAY_ADVISORY_MM: f64 = 5.0;

/// Red thread — 7-day precipitation indicating recent rain (mm).
pub const RED_THREAD_RAIN_7DAY_MM: f64 = 10.0;

// =============================================================================
// Wind speed thresholds (mph)
// =============================================================================

/// Application window — max acceptable wind speed.
pub const WIND_APP_WINDOW_MAX_MPH: f64 = 10.0;

/// Application window — calm wind threshold (bonus for scoring).
pub const WIND_CALM_MPH: f64 = 5.0;

// =============================================================================
// Time / duration thresholds
// =============================================================================

/// Days remaining in grub control window before severity escalates.
pub const GRUB_URGENCY_DAYS: i64 = 14;

/// Overseeding — days remaining that triggers "running low on time" escalation.
pub const OVERSEED_LOW_TIME_DAYS: i64 = 21;

/// Overseeding — days remaining for secondary urgency check.
pub const OVERSEED_URGENT_DAYS: i64 = 14;

/// Days between fall fertilizer applications.
pub const FALL_FERT_MIN_INTERVAL_DAYS: i64 = 21;

/// Days for nitrogen deficiency check (red thread / dollar spot risk).
pub const N_DEFICIENCY_DAYS_45: i64 = 45;

/// Extended nitrogen deficiency check (red thread severity escalation).
pub const N_DEFICIENCY_DAYS_60: i64 = 60;

/// Broadleaf herbicide — suppression window after overseeding (days).
pub const HERBICIDE_OVERSEED_BUFFER_DAYS: i64 = 60;

/// Forecast rain check window for irrigation (hours).
pub const IRRIGATION_FORECAST_HOURS: u32 = 120;

/// Rain delay — critical window (hours).
pub const RAIN_DELAY_CRITICAL_HOURS: u32 = 12;

/// Rain delay — warning window (hours).
pub const RAIN_DELAY_WARNING_HOURS: u32 = 24;

/// Rain delay — advisory window (hours).
pub const RAIN_DELAY_ADVISORY_HOURS: u32 = 48;

// =============================================================================
// Lawn defaults
// =============================================================================

/// Default lawn size (sqft) when profile doesn't specify.
pub const DEFAULT_LAWN_SIZE_SQFT: f64 = 5000.0;

/// Overseeding rate (lbs per 1000 sqft).
pub const OVERSEED_RATE_LBS_PER_KSQFT: f64 = 4.0;

/// Spring nitrogen rate (lbs N per 1000 sqft).
pub const SPRING_N_RATE_LBS_PER_KSQFT: f64 = 0.5;

/// Early fall nitrogen rate (lbs N per 1000 sqft).
pub const EARLY_FALL_N_RATE_LBS_PER_KSQFT: f64 = 1.0;

/// Mid fall nitrogen rate (lbs N per 1000 sqft).
pub const MID_FALL_N_RATE_LBS_PER_KSQFT: f64 = 0.75;

/// Winterizer nitrogen rate (lbs N per 1000 sqft).
pub const WINTERIZER_N_RATE_LBS_PER_KSQFT: f64 = 1.0;
