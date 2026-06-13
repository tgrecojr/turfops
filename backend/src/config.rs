use crate::error::{Result, TurfOpsError};
use serde::Deserialize;
use sqlx::postgres::PgConnectOptions;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub lawn: LawnConfig,
    pub noaa: NoaaConfig,
    pub datalake: DataLakeConfig,
    pub homeassistant: HomeAssistantConfig,
    pub openweathermap: Option<OpenWeatherMapConfig>,
    pub openrouter: Option<OpenRouterConfig>,
    pub server: ServerConfig,
    pub database: DatabaseConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LawnConfig {
    pub name: String,
    pub grass_type: String,
    pub usda_zone: String,
    pub soil_type: Option<String>,
    pub lawn_size_sqft: Option<f64>,
    pub irrigation_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NoaaConfig {
    pub station_wbanno: i32,
}

/// Filesystem paths to the NOAA weather data lake (parquet, produced by the
/// Dagster bronze/silver/gold pipeline and mounted into this container).
#[derive(Debug, Clone, Deserialize)]
pub struct DataLakeConfig {
    /// Silver hourly weather parquet (cleaned, deduped, one row per hourly observation).
    pub silver_weather_path: String,
    /// Gold daily weather parquet (pre-aggregated daily means + gdd50).
    pub gold_weather_path: String,
}

#[derive(Clone, Deserialize)]
pub struct HomeAssistantConfig {
    pub url: String,
    pub token: String,
    pub temperature_entity: String,
    pub humidity_entity: String,
    #[serde(default)]
    pub temperature_unit: TemperatureUnit,
}

impl std::fmt::Debug for HomeAssistantConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HomeAssistantConfig")
            .field("url", &self.url)
            .field("token", &"[REDACTED]")
            .field("temperature_entity", &self.temperature_entity)
            .field("humidity_entity", &self.humidity_entity)
            .field("temperature_unit", &self.temperature_unit)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TemperatureUnit {
    #[default]
    Fahrenheit,
    Celsius,
}

#[derive(Clone, Deserialize)]
pub struct OpenWeatherMapConfig {
    pub api_key: String,
    pub latitude: f64,
    pub longitude: f64,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

impl std::fmt::Debug for OpenWeatherMapConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenWeatherMapConfig")
            .field("api_key", &"[REDACTED]")
            .field("latitude", &self.latitude)
            .field("longitude", &self.longitude)
            .field("enabled", &self.enabled)
            .finish()
    }
}

#[derive(Clone, Deserialize)]
pub struct OpenRouterConfig {
    pub api_key: String,
    #[serde(default = "default_openrouter_model")]
    pub model: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_openrouter_base_url")]
    pub base_url: String,
}

fn default_openrouter_model() -> String {
    "anthropic/claude-haiku-4-5".to_string()
}

fn default_openrouter_base_url() -> String {
    "https://openrouter.ai/api/v1".to_string()
}

impl std::fmt::Debug for OpenRouterConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenRouterConfig")
            .field("api_key", &"[REDACTED]")
            .field("model", &self.model)
            .field("enabled", &self.enabled)
            .field("base_url", &self.base_url)
            .finish()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub cors_allowed_origin: Option<String>,
}

#[derive(Clone, Deserialize)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub name: String,
    pub user: String,
    pub password: String,
}

impl std::fmt::Debug for DatabaseConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DatabaseConfig")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("name", &self.name)
            .field("user", &self.user)
            .field("password", &"[REDACTED]")
            .finish()
    }
}

impl DatabaseConfig {
    pub fn connect_options(&self) -> PgConnectOptions {
        PgConnectOptions::new()
            .host(&self.host)
            .port(self.port)
            .database(&self.name)
            .username(&self.user)
            .password(&self.password)
    }
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

fn env_required(key: &str) -> Result<String> {
    std::env::var(key).map_err(|_| TurfOpsError::Config(format!("Missing env var: {}", key)))
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            lawn: LawnConfig {
                name: env_or("LAWN_NAME", "Main Lawn"),
                grass_type: env_or("LAWN_GRASS_TYPE", "TallFescue"),
                usda_zone: env_or("LAWN_USDA_ZONE", "7a"),
                soil_type: std::env::var("LAWN_SOIL_TYPE").ok(),
                lawn_size_sqft: std::env::var("LAWN_SIZE_SQFT")
                    .ok()
                    .and_then(|s| s.parse().ok()),
                irrigation_type: std::env::var("LAWN_IRRIGATION_TYPE").ok(),
            },
            noaa: NoaaConfig {
                station_wbanno: {
                    let raw = env_or("NOAA_STATION_WBANNO", "3761");
                    raw.parse().unwrap_or_else(|_| {
                        tracing::warn!(value = %raw, "Invalid NOAA_STATION_WBANNO, defaulting to 3761");
                        3761
                    })
                },
            },
            datalake: {
                // Either set the two full paths explicitly, or set DATALAKE_ROOT and
                // derive the conventional weather parquet locations beneath it.
                let root = env_or("DATALAKE_ROOT", "/data");
                let silver_weather_path = std::env::var("WEATHER_SILVER_PATH")
                    .unwrap_or_else(|_| format!("{root}/silver/weather/silver_weather.parquet"));
                let gold_weather_path = std::env::var("WEATHER_GOLD_PATH")
                    .unwrap_or_else(|_| format!("{root}/gold/weather/daily_weather.parquet"));
                tracing::info!(
                    silver = %silver_weather_path,
                    gold = %gold_weather_path,
                    "Weather data lake paths configured"
                );
                DataLakeConfig {
                    silver_weather_path,
                    gold_weather_path,
                }
            },
            homeassistant: HomeAssistantConfig {
                url: env_or("HA_URL", "http://localhost:8123"),
                token: env_or("HA_TOKEN", ""),
                temperature_entity: env_or(
                    "HA_TEMPERATURE_ENTITY",
                    "sensor.temp_humidity_sensor_temperature",
                ),
                humidity_entity: env_or(
                    "HA_HUMIDITY_ENTITY",
                    "sensor.temp_humidity_sensor_humidity",
                ),
                temperature_unit: if env_or("HA_TEMPERATURE_UNIT", "fahrenheit") == "celsius" {
                    TemperatureUnit::Celsius
                } else {
                    TemperatureUnit::Fahrenheit
                },
            },
            openweathermap: std::env::var("OWM_API_KEY")
                .ok()
                .map(|api_key| OpenWeatherMapConfig {
                    api_key,
                    latitude: env_or("OWM_LATITUDE", "0").parse().unwrap_or(0.0),
                    longitude: env_or("OWM_LONGITUDE", "0").parse().unwrap_or(0.0),
                    enabled: env_or("OWM_ENABLED", "true") == "true",
                }),
            openrouter: std::env::var("OPENROUTER_API_KEY")
                .ok()
                .map(|api_key| OpenRouterConfig {
                    api_key,
                    model: env_or("OPENROUTER_MODEL", "anthropic/claude-haiku-4-5"),
                    enabled: env_or("OPENROUTER_ENABLED", "true") == "true",
                    base_url: env_or("OPENROUTER_BASE_URL", "https://openrouter.ai/api/v1"),
                }),
            server: ServerConfig {
                host: env_or("SERVER_HOST", "0.0.0.0"),
                port: {
                    let raw = env_or("SERVER_PORT", "3000");
                    raw.parse().unwrap_or_else(|_| {
                        tracing::warn!(value = %raw, "Invalid SERVER_PORT, defaulting to 3000");
                        3000
                    })
                },
                cors_allowed_origin: std::env::var("CORS_ALLOWED_ORIGIN").ok(),
            },
            database: DatabaseConfig {
                host: env_or("DATABASE_HOST", "localhost"),
                port: {
                    let raw = env_or("DATABASE_PORT", "5432");
                    raw.parse().unwrap_or_else(|_| {
                        tracing::warn!(value = %raw, "Invalid DATABASE_PORT, defaulting to 5432");
                        5432
                    })
                },
                name: env_or("DATABASE_NAME", "turfops"),
                user: env_or("DATABASE_USER", "turfops"),
                password: env_required("DATABASE_PASSWORD")?,
            },
        })
    }
}
