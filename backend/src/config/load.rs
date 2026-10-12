//! Reading configuration from a variable lookup, so it can be tested without touching
//! the process environment.

use super::*;

/// A variable's value; `None` when unset.
pub(super) type Lookup<'a> = &'a dyn Fn(&str) -> Option<String>;

fn var(get: Lookup, key: &str) -> Option<String> {
    get(key)
}

fn env_or(get: Lookup, key: &str, default: &str) -> String {
    get(key).unwrap_or_else(|| default.to_string())
}

/// A setting that must be set to something; unset or blank stops startup.
// @spec DEPLOY-CONFIG-002
fn required(get: Lookup, key: &str) -> Result<String> {
    get(key)
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| TurfOpsError::Config(format!("{key} must be set (it is unset or empty)")))
}

/// A setting parsed as `T`; unset or blank uses `default`, and a value that won't parse
/// uses it too with a warning naming the setting and the default.
// @spec DEPLOY-CONFIG-012, DEPLOY-CONFIG-007
fn parsed<T: std::str::FromStr>(
    get: Lookup,
    key: &str,
    default: T,
    default_text: &str,
    warnings: &mut Vec<String>,
) -> T {
    match get(key).filter(|v| !v.trim().is_empty()) {
        None => default,
        Some(raw) => raw.trim().parse().unwrap_or_else(|_| {
            warnings.push(format!("{key}={raw:?} isn't usable; using {default_text}."));
            default
        }),
    }
}

/// `CORS_ALLOWED_ORIGIN`: blank = unset, `*` = any, otherwise exactly an http(s) origin
/// (scheme, host, optional port, nothing after). Anything else stops startup.
// @spec UI-SRV-009
fn cors_origin(raw: Option<String>) -> Result<Option<String>> {
    let Some(origin) = raw.map(|o| o.trim().to_string()).filter(|o| !o.is_empty()) else {
        return Ok(None);
    };
    if origin == "*" {
        return Ok(Some(origin));
    }
    let valid = reqwest::Url::parse(&origin).is_ok_and(|url| {
        matches!(url.scheme(), "http" | "https")
            && url.origin().ascii_serialization() == origin.trim_end_matches('/')
    });
    if valid {
        Ok(Some(origin))
    } else {
        Err(TurfOpsError::Config(format!(
            "CORS_ALLOWED_ORIGIN={origin:?} isn't an origin; use * or a value like \
             https://lawn.example.com (scheme, host and optional port, no path)"
        )))
    }
}

/// The forecast settings, when `OWM_API_KEY` is set. Without both coordinates there is
/// no lawn to forecast for: the forecast stays off with a warning naming the missing one,
/// rather than forecasting for 0, 0.
// @spec WX-SRC-CFG-003
fn forecast_config(get: Lookup, w: &mut Vec<String>) -> Option<OpenWeatherMapConfig> {
    let api_key = var(get, "OWM_API_KEY").filter(|key| !key.trim().is_empty())?;
    let missing: Vec<&str> = ["OWM_LATITUDE", "OWM_LONGITUDE"]
        .into_iter()
        .filter(|key| get(key).is_none_or(|v| v.trim().is_empty()))
        .collect();
    if !missing.is_empty() {
        w.push(format!(
            "OWM_API_KEY is set but {} isn't; the forecast is off until both coordinates are set.",
            missing.join(" and ")
        ));
        return None;
    }
    // Both are checked so each bad value gets its own warning.
    let latitude = coordinate(get, "OWM_LATITUDE", "a latitude", 90.0, w);
    let longitude = coordinate(get, "OWM_LONGITUDE", "a longitude", 180.0, w);
    Some(OpenWeatherMapConfig {
        api_key,
        latitude: latitude?,
        longitude: longitude?,
        enabled: flag(get, "OWM_ENABLED"),
    })
}

/// A set coordinate within ±`limit` degrees, or `None` with a warning: a forecast for a
/// substitute place would be worse than none.
fn coordinate(get: Lookup, key: &str, what: &str, limit: f64, w: &mut Vec<String>) -> Option<f64> {
    let raw = get(key).unwrap_or_default();
    let value = raw
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|v| (-limit..=limit).contains(v));
    if value.is_none() {
        w.push(format!(
            "{key}={raw:?} isn't {what} (-{limit} to {limit}); the forecast is off until it is."
        ));
    }
    value
}

/// A feature flag: `true`, `1` or `yes` in any case is on, anything else off, unset on.
// @spec DEPLOY-CONFIG-013
fn flag(get: Lookup, key: &str) -> bool {
    get(key).is_none_or(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "true" | "1" | "yes"))
}

/// The configuration the variables describe, and a warning for each value it could not use.
// @spec DEPLOY-CONFIG-001, DEPLOY-CONFIG-003, DEPLOY-CONFIG-008
pub fn from_vars(get: Lookup) -> Result<(Config, Vec<String>)> {
    let mut warnings: Vec<String> = Vec::new();
    let w = &mut warnings;
    let config = Config {
        lawn: LawnConfig {
            name: env_or(get, "LAWN_NAME", "Main Lawn"),
            grass_type: env_or(get, "LAWN_GRASS_TYPE", "TallFescue"),
            usda_zone: env_or(get, "LAWN_USDA_ZONE", "7a"),
            soil_type: var(get, "LAWN_SOIL_TYPE"),
            lawn_size_sqft: var(get, "LAWN_SIZE_SQFT"),
            irrigation_type: var(get, "LAWN_IRRIGATION_TYPE"),
        },
        noaa: NoaaConfig {
            station_wbanno: parsed(get, "NOAA_STATION_WBANNO", 3761, "3761", w),
        },
        datalake: {
            // Either set the two full paths explicitly, or set DATALAKE_ROOT and
            // derive the conventional weather parquet locations beneath it.
            let root = env_or(get, "DATALAKE_ROOT", "/data");
            let silver_weather_path = var(get, "WEATHER_SILVER_PATH")
                .unwrap_or_else(|| format!("{root}/silver/weather/silver_weather.parquet"));
            let gold_weather_path = var(get, "WEATHER_GOLD_PATH")
                .unwrap_or_else(|| format!("{root}/gold/weather/daily_weather.parquet"));
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
            url: env_or(get, "HA_URL", "http://localhost:8123"),
            token: env_or(get, "HA_TOKEN", ""),
            temperature_entity: env_or(
                get,
                "HA_TEMPERATURE_ENTITY",
                "sensor.temp_humidity_sensor_temperature",
            ),
            humidity_entity: env_or(
                get,
                "HA_HUMIDITY_ENTITY",
                "sensor.temp_humidity_sensor_humidity",
            ),
            temperature_unit: match var(get, "HA_TEMPERATURE_UNIT")
                .map(|v| v.trim().to_ascii_lowercase())
                .as_deref()
            {
                None | Some("") | Some("fahrenheit") => TemperatureUnit::Fahrenheit,
                Some("celsius") => TemperatureUnit::Celsius,
                Some(other) => {
                    w.push(format!(
                        "HA_TEMPERATURE_UNIT={other:?} isn't usable; using fahrenheit."
                    ));
                    TemperatureUnit::Fahrenheit
                }
            },
        },
        // docker-compose passes unset optional keys as empty strings; empty = not configured.
        openweathermap: forecast_config(get, w),
        openrouter: var(get, "OPENROUTER_API_KEY")
            .filter(|key| !key.trim().is_empty())
            .map(|api_key| OpenRouterConfig {
                api_key,
                model: env_or(get, "OPENROUTER_MODEL", DEFAULT_OPENROUTER_MODEL),
                enabled: flag(get, "OPENROUTER_ENABLED"),
                base_url: env_or(get, "OPENROUTER_BASE_URL", "https://openrouter.ai/api/v1"),
            }),
        server: ServerConfig {
            host: parsed::<std::net::IpAddr>(get, "SERVER_HOST", [0, 0, 0, 0].into(), "0.0.0.0", w)
                .to_string(),
            port: parsed(get, "SERVER_PORT", 3000, "3000", w),
            cors_allowed_origin: cors_origin(var(get, "CORS_ALLOWED_ORIGIN"))?,
        },
        database: DatabaseConfig {
            host: env_or(get, "DATABASE_HOST", "localhost"),
            port: parsed(get, "DATABASE_PORT", 5432, "5432", w),
            name: env_or(get, "DATABASE_NAME", "turfops"),
            user: env_or(get, "DATABASE_USER", "turfops"),
            password: required(get, "DATABASE_PASSWORD")?,
            max_connections: parsed(get, "DB_MAX_CONNECTIONS", 10, "10", w),
        },
        clock: crate::clock::Clock::from_tz(var(get, "TZ").as_deref())
            .map_err(TurfOpsError::Config)?,
        mcp: McpConfig {
            token: crate::mcp::parse_token(var(get, "MCP_TOKEN"))?,
        },
    };
    Ok((config, warnings))
}

/// The configuration `pairs` describe (every other variable unset), for tests.
#[cfg(test)]
pub(crate) fn test_config(pairs: &[(&str, &str)]) -> Config {
    let vars: std::collections::HashMap<String, String> = [("DATABASE_PASSWORD", "test-password")]
        .iter()
        .chain(pairs)
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    from_vars(&|key| vars.get(key).cloned()).unwrap().0
}
