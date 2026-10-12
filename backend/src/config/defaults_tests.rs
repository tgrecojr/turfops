//! Defaults, blank values and fallbacks of the environment configuration.

use super::load::from_vars;
use super::{test_config, Config, DEFAULT_OPENROUTER_MODEL};
use std::collections::HashMap;

fn load(pairs: &[(&str, &str)]) -> (Config, Vec<String>) {
    let vars: HashMap<String, String> = [("DATABASE_PASSWORD", "pw")]
        .iter()
        .chain(pairs)
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    from_vars(&|key| vars.get(key).cloned()).unwrap()
}

// @spec DEPLOY-CONFIG-001
#[test]
fn the_database_password_is_the_only_setting_without_a_default() {
    let (config, warnings) = load(&[]);
    assert!(warnings.is_empty(), "{warnings:?}");
    let db = &config.database;
    assert_eq!(
        (
            db.host.as_str(),
            db.port,
            db.name.as_str(),
            db.user.as_str()
        ),
        ("localhost", 5432, "turfops", "turfops")
    );
    assert_eq!(db.max_connections, 10);
    assert_eq!(
        (config.server.host.as_str(), config.server.port),
        ("0.0.0.0", 3000)
    );
    assert_eq!(config.server.cors_allowed_origin, None);
    assert_eq!(config.noaa.station_wbanno, 3761);
    assert_eq!(
        config.datalake.silver_weather_path,
        "/data/silver/weather/silver_weather.parquet"
    );
    assert_eq!(
        config.datalake.gold_weather_path,
        "/data/gold/weather/daily_weather.parquet"
    );
    assert_eq!(config.homeassistant.url, "http://localhost:8123");
    assert_eq!(
        (
            config.lawn.name.as_str(),
            config.lawn.grass_type.as_str(),
            config.lawn.usda_zone.as_str()
        ),
        ("Main Lawn", "TallFescue", "7a")
    );
    assert!(config.openweathermap.is_none() && config.openrouter.is_none());
    assert!(config.mcp.token.is_none());

    let none: HashMap<String, String> = HashMap::new();
    assert!(
        from_vars(&|key| none.get(key).cloned()).is_err(),
        "no default for DATABASE_PASSWORD"
    );
}

// @spec DEPLOY-CONFIG-003
#[test]
fn blank_optional_credentials_and_origin_are_not_configured() {
    for blank in ["", "   ", "\t\n"] {
        let (config, warnings) = load(&[
            ("OPENROUTER_API_KEY", blank),
            ("MCP_TOKEN", blank),
            ("CORS_ALLOWED_ORIGIN", blank),
        ]);
        assert!(config.openrouter.is_none(), "{blank:?}");
        assert!(config.mcp.token.is_none(), "{blank:?}");
        assert!(config.server.cors_allowed_origin.is_none(), "{blank:?}");
        assert!(warnings.is_empty(), "{blank:?}: {warnings:?}");
    }
}

// @spec DEPLOY-CONFIG-004
#[test]
fn the_openrouter_settings_come_from_the_environment_with_defaults() {
    let (config, _) = load(&[("OPENROUTER_API_KEY", "sk-or-test")]);
    let or = config.openrouter.expect("a key configures OpenRouter");
    assert_eq!(or.model, DEFAULT_OPENROUTER_MODEL);
    assert_eq!(or.base_url, "https://openrouter.ai/api/v1");
    assert!(or.enabled, "on by default");

    let (config, _) = load(&[
        ("OPENROUTER_API_KEY", "sk-or-test"),
        ("OPENROUTER_MODEL", "openai/gpt-5-mini"),
        ("OPENROUTER_BASE_URL", "http://127.0.0.1:9/api/v1"),
        ("OPENROUTER_ENABLED", "false"),
    ]);
    let or = config.openrouter.unwrap();
    assert_eq!(or.model, "openai/gpt-5-mini");
    assert_eq!(or.base_url, "http://127.0.0.1:9/api/v1");
    assert!(!or.enabled);

    let (config, _) = load(&[("OPENROUTER_MODEL", "openai/gpt-5-mini")]);
    assert!(config.openrouter.is_none(), "no key, no client");
}

// @spec DEPLOY-CONFIG-007
#[test]
fn an_unparseable_port_or_grass_type_warns_and_uses_the_default() {
    let (config, warnings) = load(&[("DATABASE_PORT", "five432"), ("SERVER_PORT", "http")]);
    assert_eq!((config.database.port, config.server.port), (5432, 3000));
    for (var, value, default) in [
        ("DATABASE_PORT", "five432", "5432"),
        ("SERVER_PORT", "http", "3000"),
    ] {
        assert!(
            warnings
                .iter()
                .any(|w| w.contains(var) && w.contains(value) && w.contains(default)),
            "{var}: {warnings:?}"
        );
    }

    let config = test_config(&[("LAWN_GRASS_TYPE", "Kentucky Blue")]);
    let (profile, warnings) = config.lawn.seed_profile();
    assert_eq!(profile.grass_type, crate::models::GrassType::TallFescue);
    assert!(
        warnings.iter().any(|w| w.contains("LAWN_GRASS_TYPE")
            && w.contains("Kentucky Blue")
            && w.contains("TallFescue")),
        "{warnings:?}"
    );
}

// @spec DEPLOY-CONFIG-008
#[test]
fn the_pool_size_comes_from_db_max_connections_defaulting_to_10() {
    assert_eq!(load(&[]).0.database.max_connections, 10);
    let (config, warnings) = load(&[("DB_MAX_CONNECTIONS", "25")]);
    assert_eq!(config.database.max_connections, 25);
    assert!(warnings.is_empty(), "{warnings:?}");
}
