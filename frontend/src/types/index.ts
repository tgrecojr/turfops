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
