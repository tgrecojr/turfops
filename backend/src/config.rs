use crate::error::{Result, TurfOpsError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub lawn: LawnConfig,
    pub noaa: NoaaConfig,
    pub soildata: SoilDataConfig,
    pub homeassistant: HomeAssistantConfig,
    pub openweathermap: Option<OpenWeatherMapConfig>,
    pub server: ServerConfig,
    pub database: DatabaseConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LawnConfig {
    pub name: String,
    pub grass_type: String,
    pub usda_zone: String,
    pub soil_type: Option<String>,
    pub lawn_size_sqft: Option<f64>,
    pub irrigation_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NoaaConfig {
    pub station_wbanno: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SoilDataConfig {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub user: String,
    pub password: String,
}

impl SoilDataConfig {
    pub fn connection_string(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.user, self.password, self.host, self.port, self.database
        )
    }
}

#[derive(Clone, Deserialize, Serialize)]
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

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TemperatureUnit {
    #[default]
    Fahrenheit,
    Celsius,
}

#[derive(Clone, Deserialize, Serialize)]
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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub name: String,
    pub user: String,
    pub password: String,
}

impl DatabaseConfig {
    pub fn connection_string(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.user, self.password, self.host, self.port, self.name
        )
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
            soildata: SoilDataConfig {
                host: env_or("SOILDATA_DB_HOST", "localhost"),
                port: {
                    let raw = env_or("SOILDATA_DB_PORT", "5432");
                    raw.parse().unwrap_or_else(|_| {
                        tracing::warn!(value = %raw, "Invalid SOILDATA_DB_PORT, defaulting to 5432");
                        5432
                    })
                },
                database: env_or("SOILDATA_DB_NAME", "uscrn"),
                user: env_or("SOILDATA_DB_USER", "postgres"),
                password: env_or("SOILDATA_DB_PASSWORD", ""),
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
            server: ServerConfig {
                host: env_or("SERVER_HOST", "0.0.0.0"),
                port: {
                    let raw = env_or("SERVER_PORT", "3000");
                    raw.parse().unwrap_or_else(|_| {
                        tracing::warn!(value = %raw, "Invalid SERVER_PORT, defaulting to 3000");
                        3000
                    })
                },
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
