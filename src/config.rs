use crate::error::{Result, TurfOpsError};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub lawn: LawnConfig,
    pub noaa: NoaaConfig,
    pub soildata: SoilDataConfig,
    pub homeassistant: HomeAssistantConfig,
    pub refresh: RefreshConfig,
    pub display: DisplayConfig,
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
    pub station_name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SoilDataConfig {
    pub host: String,
    #[serde(deserialize_with = "deserialize_port")]
    pub port: u16,
    pub database: String,
    pub user: String,
    pub password: String,
}

fn deserialize_port<'de, D>(deserializer: D) -> std::result::Result<u16, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    let value = String::deserialize(deserializer)?;
    value.parse::<u16>().map_err(|_| {
        D::Error::custom(format!(
            "invalid port '{}' - ensure SOILDATA_DB_PORT environment variable is set",
            value
        ))
    })
}

impl SoilDataConfig {
    pub fn connection_string(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.user, self.password, self.host, self.port, self.database
        )
    }
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

#[derive(Debug, Clone, Deserialize)]
pub struct RefreshConfig {
    pub environmental_interval_minutes: u32,
    pub cache_duration_hours: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DisplayConfig {
    pub temperature_unit: String,
    pub date_format: String,
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if !config_path.exists() {
            return Err(TurfOpsError::Config(format!(
                "Config file not found at {:?}. Copy config/config.yaml.example to config/config.yaml",
                config_path
            )));
        }

        let config_str = std::fs::read_to_string(&config_path)
            .map_err(|e| TurfOpsError::Config(format!("Failed to read config: {}", e)))?;

        // Substitute environment variables
        let config_str = Self::substitute_env_vars(&config_str);

        let config: Config = serde_yaml::from_str(&config_str)
            .map_err(|e| TurfOpsError::Config(format!("Failed to parse config: {}", e)))?;

        Ok(config)
    }

    fn config_path() -> Result<PathBuf> {
        // Try current directory first
        let local_config = PathBuf::from("config/config.yaml");
        if local_config.exists() {
            return Ok(local_config);
        }

        // Try XDG config directory
        if let Some(config_dir) = dirs::config_dir() {
            let xdg_config = config_dir.join("turfops").join("config.yaml");
            if xdg_config.exists() {
                return Ok(xdg_config);
            }
        }

        // Return local path (will trigger "not found" error)
        Ok(local_config)
    }

    fn substitute_env_vars(content: &str) -> String {
        let mut result = content.to_string();

        // Find all ${VAR_NAME} patterns and substitute
        let re = regex_lite::Regex::new(r"\$\{([A-Z_][A-Z0-9_]*)\}").unwrap();

        for cap in re.captures_iter(content) {
            let var_name = &cap[1];
            let placeholder = &cap[0];
            if let Ok(value) = std::env::var(var_name) {
                result = result.replace(placeholder, &value);
            }
        }

        result
    }

    pub fn data_dir() -> Result<PathBuf> {
        // Check for override
        if let Ok(dir) = std::env::var("TURFOPS_DATA_DIR") {
            return Ok(PathBuf::from(dir));
        }

        // Use XDG data directory
        let data_dir = dirs::data_dir()
            .ok_or_else(|| TurfOpsError::Config("Cannot determine data directory".into()))?
            .join("turfops");

        std::fs::create_dir_all(&data_dir)?;
        Ok(data_dir)
    }

    pub fn db_path() -> Result<PathBuf> {
        Ok(Self::data_dir()?.join("turfops.db"))
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            lawn: LawnConfig {
                name: "Main Lawn".into(),
                grass_type: "TallFescue".into(),
                usda_zone: "7a".into(),
                soil_type: Some("Loam".into()),
                lawn_size_sqft: Some(5000.0),
                irrigation_type: Some("InGround".into()),
            },
            noaa: NoaaConfig {
                station_wbanno: 3761,
                station_name: "PA Avondale".into(),
            },
            soildata: SoilDataConfig {
                host: "localhost".into(),
                port: 5432,
                database: "uscrn".into(),
                user: "postgres".into(),
                password: "".into(),
            },
            homeassistant: HomeAssistantConfig {
                url: "http://localhost:8123".into(),
                token: "".into(),
                temperature_entity: "sensor.temp_humidity_sensor_temperature".into(),
                humidity_entity: "sensor.temp_humidity_sensor_humidity".into(),
                temperature_unit: TemperatureUnit::Fahrenheit,
            },
            refresh: RefreshConfig {
                environmental_interval_minutes: 15,
                cache_duration_hours: 24,
            },
            display: DisplayConfig {
                temperature_unit: "fahrenheit".into(),
                date_format: "%Y-%m-%d".into(),
            },
        }
    }
}
