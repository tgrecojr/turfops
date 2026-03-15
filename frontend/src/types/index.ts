// Matches Rust models exactly

export interface LawnProfile {
  id: number | null;
  name: string;
  grass_type: GrassType;
  usda_zone: string;
  soil_type: SoilType | null;
  lawn_size_sqft: number | null;
  irrigation_type: IrrigationType | null;
  created_at: string;
  updated_at: string;
}

export type GrassType =
  | 'KentuckyBluegrass'
  | 'TallFescue'
  | 'PerennialRyegrass'
  | 'FineFescue'
  | 'Bermuda'
  | 'Zoysia'
  | 'StAugustine'
  | 'Mixed';

export type SoilType =
  | 'Clay'
  | 'Loam'
  | 'Sandy'
  | 'SiltLoam'
  | 'ClayLoam'
  | 'SandyLoam';

export type IrrigationType = 'InGround' | 'Hose' | 'None';

export interface Application {
  id: number | null;
  lawn_profile_id: number;
  application_type: ApplicationType;
  product_name: string | null;
  application_date: string;
  rate_per_1000sqft: number | null;
  coverage_sqft: number | null;
  notes: string | null;
  weather_snapshot: WeatherSnapshot | null;
  nitrogen_pct: number | null;
  phosphorus_pct: number | null;
  potassium_pct: number | null;
  created_at: string;
}

export type ApplicationType =
  | 'PreEmergent'
  | 'PostEmergent'
  | 'Fertilizer'
  | 'Fungicide'
  | 'Insecticide'
  | 'GrubControl'
  | 'Overseed'
  | 'Aeration'
  | 'Dethatching'
  | 'Lime'
  | 'Sulfur'
  | 'Wetting'
  | 'Other';

export interface WeatherSnapshot {
  soil_temp_10cm_f: number | null;
  ambient_temp_f: number | null;
  humidity_percent: number | null;
  soil_moisture: number | null;
}

export interface EnvironmentalSummary {
  current: EnvironmentalReading | null;
  soil_temp_7day_avg_f: number | null;
  ambient_temp_7day_avg_f: number | null;
  humidity_7day_avg: number | null;
  precipitation_7day_total_mm: number | null;
  soil_temp_trend: Trend;
  last_updated: string | null;
  forecast: WeatherForecast | null;
  gdd_base50_ytd: number | null;
}

export interface EnvironmentalReading {
  timestamp: string;
  source: string;
  soil_temp_5_f: number | null;
  soil_temp_10_f: number | null;
  soil_temp_20_f: number | null;
  soil_temp_50_f: number | null;
  soil_temp_100_f: number | null;
  soil_moisture_5: number | null;
  soil_moisture_10: number | null;
  soil_moisture_20: number | null;
  soil_moisture_50: number | null;
  soil_moisture_100: number | null;
  ambient_temp_f: number | null;
  humidity_percent: number | null;
  precipitation_mm: number | null;
}

export type Trend = 'Rising' | 'Falling' | 'Stable' | 'Unknown';

export interface WeatherForecast {
  fetched_at: string;
  location: ForecastLocation;
  hourly: ForecastPoint[];
  daily_summary: DailyForecast[];
}

export interface ForecastLocation {
  city: string;
  country: string;
  latitude: number;
  longitude: number;
}

export interface ForecastPoint {
  timestamp: string;
  temp_f: number;
  feels_like_f: number;
  humidity_percent: number;
  precipitation_mm: number;
  precipitation_prob: number;
  wind_speed_mph: number;
  wind_gust_mph: number | null;
  cloud_cover_percent: number;
  weather_condition: string;
}

export interface DailyForecast {
  date: string;
  high_temp_f: number;
  low_temp_f: number;
  avg_humidity: number;
  total_precipitation_mm: number;
  max_precipitation_prob: number;
  dominant_condition: string;
  avg_wind_speed_mph: number;
  max_wind_gust_mph: number | null;
}

export interface Recommendation {
  id: string;
  category: string;
  severity: Severity;
  title: string;
  description: string;
  explanation: string;
  data_points: DataPoint[];
  suggested_action: string | null;
  created_at: string;
  dismissed: boolean;
  addressed: boolean;
}

export type Severity = 'Info' | 'Advisory' | 'Warning' | 'Critical';

export interface DataPoint {
  label: string;
  value: string;
  source: string;
}

export interface ConnectionStatus {
  soildata: boolean;
  homeassistant: boolean;
  openweathermap: boolean;
}

export interface HealthResponse {
  status: string;
  version: string;
  database: boolean;
  datasources: ConnectionStatus;
}

export interface DashboardResponse {
  profile: LawnProfile;
  environmental: EnvironmentalSummary;
  recommendations: Recommendation[];
  recent_applications: Application[];
  connections: ConnectionStatus;
}

export interface CalendarResponse {
  year: number;
  month: number;
  days: Record<string, Application[]>;
}

// GDD types

export interface DailyGdd {
  date: string;
  high_temp_f: number;
  low_temp_f: number;
  gdd_base50: number;
  cumulative_gdd_base50: number;
}

export type CrabgrassStatus =
  | 'PreGermination'
  | 'ApproachingGermination'
  | 'GerminationLikely'
  | 'PostGermination';

export interface CrabgrassModel {
  germination_threshold: number;
  current_gdd: number;
  status: CrabgrassStatus;
  estimated_germination_date: string | null;
}

export interface GddSummary {
  year: number;
  current_gdd_total: number;
  daily_history: DailyGdd[];
  crabgrass_model: CrabgrassModel;
  last_computed_date: string | null;
}

// Historical time-series types

export interface TimeSeriesPoint {
  timestamp: string;
  value: number;
}

export interface HistoricalData {
  range: string;
  soil_temp_10_f: TimeSeriesPoint[];
  ambient_temp_f: TimeSeriesPoint[];
  humidity_percent: TimeSeriesPoint[];
  soil_moisture_10: TimeSeriesPoint[];
  precipitation_mm: TimeSeriesPoint[];
  gdd_accumulation: TimeSeriesPoint[];
}

// Nitrogen budget types

export interface NitrogenApplication {
  date: string;
  product_name: string | null;
  nitrogen_pct: number;
  rate_per_1000sqft: number;
  n_lbs_per_1000sqft: number;
}

export interface GrassTypeNTarget {
  grass_type: GrassType;
  min_lbs_per_1000sqft: number;
  max_lbs_per_1000sqft: number;
  recommended_lbs_per_1000sqft: number;
}

export interface NitrogenBudget {
  year: number;
  target_lbs_per_1000sqft: number;
  applied_lbs_per_1000sqft: number;
  remaining_lbs_per_1000sqft: number;
  percent_of_target: number;
  applications: NitrogenApplication[];
  grass_type_target: GrassTypeNTarget;
}

export const CRABGRASS_STATUS_LABELS: Record<CrabgrassStatus, string> = {
  PreGermination: 'Pre-Germination',
  ApproachingGermination: 'Approaching',
  GerminationLikely: 'Likely',
  PostGermination: 'Post-Germination',
};

export const CRABGRASS_STATUS_COLORS: Record<CrabgrassStatus, string> = {
  PreGermination: '#48bb78',
  ApproachingGermination: '#eab308',
  GerminationLikely: '#f97316',
  PostGermination: '#ef4444',
};

// Seasonal Plan types

export interface SeasonalPlan {
  year: number;
  activities: PlannedActivity[];
  data_years_used: number;
  generated_at: string;
}

export interface PlannedActivity {
  id: string;
  name: string;
  category: string;
  description: string;
  date_window: DateWindow;
  status: ActivityStatus;
  details: ActivityDetails;
}

export interface DateWindow {
  predicted_start: string;
  predicted_end: string;
  earliest_historical: string | null;
  latest_historical: string | null;
  confidence: WindowConfidence;
}

export type WindowConfidence = 'High' | 'Medium' | 'Low';

export type ActivityStatus = 'Upcoming' | 'Active' | 'Completed' | 'Missed';

export interface ActivityDetails {
  soil_temp_trigger: string | null;
  product_suggestions: string[];
  rate: string | null;
  notes: string | null;
}

export const ACTIVITY_STATUS_COLORS: Record<ActivityStatus, string> = {
  Upcoming: '#3b82f6',
  Active: '#22c55e',
  Completed: '#6b7280',
  Missed: '#ef4444',
};

export const CONFIDENCE_LABELS: Record<WindowConfidence, string> = {
  High: 'High confidence',
  Medium: 'Moderate confidence',
  Low: 'Low confidence',
};

// Display helpers

export const APPLICATION_TYPE_LABELS: Record<ApplicationType, string> = {
  PreEmergent: 'Pre-Emergent',
  PostEmergent: 'Post-Emergent',
  Fertilizer: 'Fertilizer',
  Fungicide: 'Fungicide',
  Insecticide: 'Insecticide',
  GrubControl: 'Grub Control',
  Overseed: 'Overseed',
  Aeration: 'Aeration',
  Dethatching: 'Dethatching',
  Lime: 'Lime',
  Sulfur: 'Sulfur',
  Wetting: 'Wetting Agent',
  Other: 'Other',
};

export const APPLICATION_TYPE_COLORS: Record<ApplicationType, string> = {
  PreEmergent: '#eab308',
  PostEmergent: '#facc15',
  Fertilizer: '#22c55e',
  Fungicide: '#d946ef',
  Insecticide: '#ef4444',
  GrubControl: '#f87171',
  Overseed: '#06b6d4',
  Aeration: '#3b82f6',
  Dethatching: '#60a5fa',
  Lime: '#f5f5f5',
  Sulfur: '#facc15',
  Wetting: '#67e8f9',
  Other: '#9ca3af',
};

export const SEVERITY_COLORS: Record<Severity, string> = {
  Info: '#9ca3af',
  Advisory: '#3b82f6',
  Warning: '#eab308',
  Critical: '#ef4444',
};

export const SEVERITY_SYMBOLS: Record<Severity, string> = {
  Info: 'ℹ',
  Advisory: '→',
  Warning: '⚠',
  Critical: '!',
};

export const GRASS_TYPE_LABELS: Record<GrassType, string> = {
  KentuckyBluegrass: 'Kentucky Bluegrass',
  TallFescue: 'Tall Fescue',
  PerennialRyegrass: 'Perennial Ryegrass',
  FineFescue: 'Fine Fescue',
  Bermuda: 'Bermuda',
  Zoysia: 'Zoysia',
  StAugustine: 'St. Augustine',
  Mixed: 'Mixed',
};
