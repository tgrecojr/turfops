use crate::error::{Result, TurfOpsError};
use dialoguer::{Input, Password};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub lawn: LawnConfig,
    pub noaa: NoaaConfig,
    pub soildata: SoilDataConfig,
    pub homeassistant: HomeAssistantConfig,
    pub openweathermap: Option<OpenWeatherMapConfig>,
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

impl Config {
    pub fn load(config_override: Option<PathBuf>) -> Result<Self> {
        let config_path = match config_override {
            Some(p) => p,
            None => Self::find_config_path()?,
        };

        if !config_path.exists() {
            return Err(TurfOpsError::Config(format!(
                "Config file not found at {:?}. Run `turfops init` to set up.",
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

    /// Search for config.yaml in standard locations.
    /// Returns the path of the first found config, or the XDG default path if none found.
    fn find_config_path() -> Result<PathBuf> {
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

        // Return XDG path as the default (will trigger "not found" in load)
        let default_path = dirs::config_dir()
            .ok_or_else(|| TurfOpsError::Config("Cannot determine config directory".into()))?
            .join("turfops")
            .join("config.yaml");
        Ok(default_path)
    }

    /// Returns true if a config file can be found in any standard location.
    pub fn exists(config_override: Option<&PathBuf>) -> bool {
        match config_override {
            Some(p) => p.exists(),
            None => Self::find_config_path()
                .map(|p| p.exists())
                .unwrap_or(false),
        }
    }

    /// Default path for writing new config files (~/.config/turfops/config.yaml).
    pub fn default_config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| TurfOpsError::Config("Cannot determine config directory".into()))?
            .join("turfops");
        Ok(config_dir.join("config.yaml"))
    }

    /// Run interactive setup prompts and write config to disk.
    /// Returns the loaded Config and the path it was written to.
    pub fn setup_interactive() -> Result<(Self, PathBuf)> {
        println!();
        println!("No configuration found. Let's set up TurfOps!");
        println!();

        // --- Lawn Profile ---
        println!("Lawn Profile");
        let lawn_name: String = Input::new()
            .with_prompt("  Lawn name")
            .default("Main Lawn".into())
            .interact_text()
            .map_err(|e| TurfOpsError::Config(format!("Input error: {}", e)))?;

        let grass_type: String = Input::new()
            .with_prompt("  Grass type (TallFescue, KBG, Bermuda, ...)")
            .default("TallFescue".into())
            .interact_text()
            .map_err(|e| TurfOpsError::Config(format!("Input error: {}", e)))?;

        let usda_zone: String = Input::new()
            .with_prompt("  USDA zone")
            .default("7a".into())
            .interact_text()
            .map_err(|e| TurfOpsError::Config(format!("Input error: {}", e)))?;

        println!();

        // --- SoilData PostgreSQL ---
        println!("SoilData PostgreSQL");
        let sd_host: String = Input::new()
            .with_prompt("  Host")
            .default("localhost".into())
            .interact_text()
            .map_err(|e| TurfOpsError::Config(format!("Input error: {}", e)))?;

        let sd_port: u16 = Input::new()
            .with_prompt("  Port")
            .default(5432)
            .interact_text()
            .map_err(|e| TurfOpsError::Config(format!("Input error: {}", e)))?;

        let sd_database: String = Input::new()
            .with_prompt("  Database")
            .default("uscrn".into())
            .interact_text()
            .map_err(|e| TurfOpsError::Config(format!("Input error: {}", e)))?;

        let sd_user: String = Input::new()
            .with_prompt("  User")
            .default("postgres".into())
            .interact_text()
            .map_err(|e| TurfOpsError::Config(format!("Input error: {}", e)))?;

        let sd_password: String = Password::new()
            .with_prompt("  Password")
            .allow_empty_password(true)
            .interact()
            .map_err(|e| TurfOpsError::Config(format!("Input error: {}", e)))?;

        println!();

        // --- Home Assistant (optional) ---
        println!("Home Assistant (leave URL blank to skip)");
        let ha_url: String = Input::new()
            .with_prompt("  URL")
            .default(String::new())
            .allow_empty(true)
            .interact_text()
            .map_err(|e| TurfOpsError::Config(format!("Input error: {}", e)))?;

        let (ha_token, ha_temp_entity, ha_humidity_entity) = if ha_url.is_empty() {
            (String::new(), String::new(), String::new())
        } else {
            let token: String = Password::new()
                .with_prompt("  Token")
                .allow_empty_password(true)
                .interact()
                .map_err(|e| TurfOpsError::Config(format!("Input error: {}", e)))?;

            let temp_entity: String = Input::new()
                .with_prompt("  Temperature entity")
                .default("sensor.temp_humidity_sensor_temperature".into())
                .interact_text()
                .map_err(|e| TurfOpsError::Config(format!("Input error: {}", e)))?;

            let humidity_entity: String = Input::new()
                .with_prompt("  Humidity entity")
                .default("sensor.temp_humidity_sensor_humidity".into())
                .interact_text()
                .map_err(|e| TurfOpsError::Config(format!("Input error: {}", e)))?;

            (token, temp_entity, humidity_entity)
        };

        println!();

        // --- OpenWeatherMap (optional) ---
        println!("OpenWeatherMap (leave API key blank to skip)");
        let owm_api_key: String = Input::new()
            .with_prompt("  API key")
            .default(String::new())
            .allow_empty(true)
            .interact_text()
            .map_err(|e| TurfOpsError::Config(format!("Input error: {}", e)))?;

        let openweathermap = if owm_api_key.is_empty() {
            None
        } else {
            let latitude: f64 = Input::new()
                .with_prompt("  Latitude")
                .default(39.83)
                .interact_text()
                .map_err(|e| TurfOpsError::Config(format!("Input error: {}", e)))?;

            let longitude: f64 = Input::new()
                .with_prompt("  Longitude")
                .default(-75.87)
                .interact_text()
                .map_err(|e| TurfOpsError::Config(format!("Input error: {}", e)))?;

            Some(OpenWeatherMapConfig {
                api_key: owm_api_key,
                latitude,
                longitude,
                enabled: true,
            })
        };

        println!();

        let config = Config {
            lawn: LawnConfig {
                name: lawn_name,
                grass_type,
                usda_zone,
                soil_type: Some("Loam".into()),
                lawn_size_sqft: Some(5000.0),
                irrigation_type: Some("InGround".into()),
            },
            noaa: NoaaConfig {
                station_wbanno: 3761,
            },
            soildata: SoilDataConfig {
                host: sd_host,
                port: sd_port,
                database: sd_database,
                user: sd_user,
                password: sd_password,
            },
            homeassistant: HomeAssistantConfig {
                url: ha_url,
                token: ha_token,
                temperature_entity: ha_temp_entity,
                humidity_entity: ha_humidity_entity,
                temperature_unit: TemperatureUnit::Fahrenheit,
            },
            openweathermap,
        };

        // Write to default config path
        let config_path = Self::default_config_path()?;
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let yaml = serde_yaml::to_string(&config)
            .map_err(|e| TurfOpsError::Config(format!("Failed to serialize config: {}", e)))?;

        // Write with a header comment
        let content = format!(
            "# TurfOps Configuration\n# Generated by `turfops init`\n# Environment variable substitution (${{VAR}}) is supported.\n\n{}",
            yaml
        );
        std::fs::write(&config_path, content)?;

        println!("Configuration saved to {}", config_path.display());
        println!();

        Ok((config, config_path))
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

    pub fn data_dir(data_dir_override: Option<&PathBuf>) -> Result<PathBuf> {
        // CLI override takes priority
        if let Some(dir) = data_dir_override {
            std::fs::create_dir_all(dir)?;
            return Ok(dir.clone());
        }

        // Then check env var
        if let Ok(dir) = std::env::var("TURFOPS_DATA_DIR") {
            let p = PathBuf::from(dir);
            std::fs::create_dir_all(&p)?;
            return Ok(p);
        }

        // Use XDG data directory
        let data_dir = dirs::data_dir()
            .ok_or_else(|| TurfOpsError::Config("Cannot determine data directory".into()))?
            .join("turfops");

        std::fs::create_dir_all(&data_dir)?;
        Ok(data_dir)
    }

    pub fn db_path(data_dir_override: Option<&PathBuf>) -> Result<PathBuf> {
        Ok(Self::data_dir(data_dir_override)?.join("turfops.db"))
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
            openweathermap: None,
        }
    }
}
