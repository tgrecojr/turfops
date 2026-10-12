use crate::error::{Result, TurfOpsError};
pub use crate::mcp::McpConfig;
use serde::Deserialize;
use sqlx::postgres::PgConnectOptions;

mod load;
#[cfg(test)]
mod load_tests;
#[cfg(all(test, feature = "db-tests"))]
mod startup_profile_db_tests;

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
    pub mcp: McpConfig,
    /// The lawn's clock, resolved from `TZ`.
    #[serde(skip)]
    pub clock: crate::clock::Clock,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LawnConfig {
    pub name: String,
    pub grass_type: String,
    pub usda_zone: String,
    pub soil_type: Option<String>,
    /// `LAWN_SIZE_SQFT` as given; parsed (with a warning when invalid) when seeding.
    pub lawn_size_sqft: Option<String>,
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

/// The model the plan and product assistants use when `OPENROUTER_MODEL` is unset.
// OpenRouter ids are dotted (`4.5`); the dashed form is rejected.
// @spec PLANT-AI-010
pub const DEFAULT_OPENROUTER_MODEL: &str = "anthropic/claude-haiku-4.5";

fn default_openrouter_model() -> String {
    DEFAULT_OPENROUTER_MODEL.to_string()
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
    /// `DB_MAX_CONNECTIONS` (default 10).
    pub max_connections: u32,
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

impl Config {
    pub fn from_env() -> Result<Self> {
        let (config, warnings) = load::from_vars(&|key| std::env::var(key).ok())?;
        for warning in warnings {
            tracing::warn!("{warning}");
        }
        Ok(config)
    }
}

impl LawnConfig {
    /// The lawn profile the `LAWN_*` settings describe, and a warning for each value that
    /// could not be used.
    // @spec APP-PROF-006
    pub fn seed_profile(&self) -> (crate::models::LawnProfile, Vec<String>) {
        use crate::models::{GrassType, IrrigationType, LawnProfile, SoilType};
        use std::str::FromStr;
        let mut warnings = Vec::new();
        let mut warn = |var: &str, value: &str, fallback: &str| {
            warnings.push(format!("{var}={value:?} isn't a valid value; {fallback}."));
        };

        let grass_type = GrassType::from_str(&self.grass_type).unwrap_or_else(|_| {
            warn("LAWN_GRASS_TYPE", &self.grass_type, "using TallFescue");
            GrassType::TallFescue
        });
        let mut profile = LawnProfile::new(self.name.clone(), grass_type, self.usda_zone.clone());
        if let Some(raw) = &self.soil_type {
            profile.soil_type = SoilType::from_str(raw).ok();
            if profile.soil_type.is_none() {
                warn("LAWN_SOIL_TYPE", raw, "leaving the soil type unset");
            }
        }
        if let Some(raw) = &self.irrigation_type {
            profile.irrigation_type = IrrigationType::from_str(raw).ok();
            if profile.irrigation_type.is_none() {
                warn(
                    "LAWN_IRRIGATION_TYPE",
                    raw,
                    "leaving the irrigation type unset",
                );
            }
        }
        if let Some(raw) = &self.lawn_size_sqft {
            profile.lawn_size_sqft = raw.trim().parse::<f64>().ok().filter(|n| *n > 0.0);
            if profile.lawn_size_sqft.is_none() {
                warn("LAWN_SIZE_SQFT", raw, "leaving the lawn size unset");
            }
        }
        (profile, warnings)
    }
}

#[cfg(test)]
mod seed_tests {
    use super::*;
    use crate::models::GrassType;

    fn lawn() -> LawnConfig {
        LawnConfig {
            name: "Main Lawn".into(),
            grass_type: "TallFescue".into(),
            usda_zone: "7a".into(),
            soil_type: Some("Clay".into()),
            lawn_size_sqft: Some("5000".into()),
            irrigation_type: Some("InGround".into()),
        }
    }

    // @spec APP-PROF-006
    #[test]
    fn each_unusable_lawn_setting_is_named_and_skipped() {
        let (profile, warnings) = lawn().seed_profile();
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(profile.lawn_size_sqft, Some(5000.0));

        let bad = LawnConfig {
            grass_type: "Bluegras".into(),
            soil_type: Some("Mud".into()),
            lawn_size_sqft: Some("5,000 sq ft".into()),
            irrigation_type: Some("Sprinklers".into()),
            ..lawn()
        };
        let (profile, warnings) = bad.seed_profile();
        assert_eq!(profile.grass_type, GrassType::TallFescue);
        assert_eq!(
            (
                profile.soil_type,
                profile.irrigation_type,
                profile.lawn_size_sqft
            ),
            (None, None, None)
        );
        for (var, value) in [
            ("LAWN_GRASS_TYPE", "Bluegras"),
            ("LAWN_SOIL_TYPE", "Mud"),
            ("LAWN_SIZE_SQFT", "5,000 sq ft"),
            ("LAWN_IRRIGATION_TYPE", "Sprinklers"),
        ] {
            assert!(
                warnings
                    .iter()
                    .any(|w| w.contains(var) && w.contains(value)),
                "{var}: {warnings:?}"
            );
        }
    }
}

#[cfg(test)]
mod openrouter_default_tests {
    // @spec PLANT-AI-010
    #[test]
    fn the_default_model_is_openrouters_dotted_id_everywhere() {
        assert_eq!(
            super::DEFAULT_OPENROUTER_MODEL,
            "anthropic/claude-haiku-4.5"
        );
        let compose = include_str!("../../docker-compose.yml");
        assert!(
            compose.contains("OPENROUTER_MODEL:-anthropic/claude-haiku-4.5}"),
            "compose default"
        );
    }
}
