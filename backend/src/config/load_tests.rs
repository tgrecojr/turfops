use super::load::from_vars;
use super::Config;
use crate::error::TurfOpsError;
use std::collections::HashMap;

const DB_SECRET: &str = "s3cret-db-pass";
const HA_SECRET: &str = "ha-long-lived-token";
const OWM_SECRET: &str = "owm-key-123";
const OR_SECRET: &str = "sk-or-v1-abc";
const MCP_SECRET: &str = "0123456789abcdef0123456789abcdef";

fn base() -> HashMap<&'static str, String> {
    HashMap::from([("DATABASE_PASSWORD", DB_SECRET.to_string())])
}

fn load(vars: &HashMap<&'static str, String>) -> crate::error::Result<(Config, Vec<String>)> {
    from_vars(&|key| vars.get(key).cloned())
}

fn with(pairs: &[(&'static str, &str)]) -> HashMap<&'static str, String> {
    let mut vars = base();
    for (k, v) in pairs {
        vars.insert(k, v.to_string());
    }
    vars
}

// @spec DEPLOY-CONFIG-002
#[test]
fn a_missing_or_empty_database_password_stops_startup() {
    for vars in [
        HashMap::new(),
        with(&[("DATABASE_PASSWORD", "")]),
        with(&[("DATABASE_PASSWORD", "  ")]),
    ] {
        match load(&vars) {
            Err(TurfOpsError::Config(msg)) => assert!(msg.contains("DATABASE_PASSWORD"), "{msg}"),
            other => panic!("expected a configuration error, got ok={}", other.is_ok()),
        }
    }
    assert!(load(&base()).is_ok());
}

// @spec DEPLOY-CONFIG-012
#[test]
fn an_unusable_setting_is_named_with_the_default_used() {
    let vars = with(&[
        ("SERVER_HOST", "my-laptop"),
        ("DB_MAX_CONNECTIONS", "lots"),
        ("HA_TEMPERATURE_UNIT", "kelvin"),
    ]);
    let (config, warnings) = load(&vars).unwrap();
    assert_eq!(config.server.host, "0.0.0.0");
    assert_eq!(config.database.max_connections, 10);
    for (var, default) in [
        ("SERVER_HOST", "0.0.0.0"),
        ("DB_MAX_CONNECTIONS", "10"),
        ("HA_TEMPERATURE_UNIT", "fahrenheit"),
    ] {
        assert!(
            warnings
                .iter()
                .any(|w| w.contains(var) && w.contains(default)),
            "{var}: {warnings:?}"
        );
    }
    let (_, none) = load(&with(&[
        ("SERVER_HOST", "127.0.0.1"),
        ("HA_TEMPERATURE_UNIT", "Celsius"),
    ]))
    .unwrap();
    assert!(none.is_empty(), "{none:?}");
}

// @spec DEPLOY-CONFIG-013
#[test]
fn feature_flags_accept_true_1_or_yes_in_any_case() {
    let enabled = |value: Option<&str>| {
        let mut vars = with(&[
            ("OWM_API_KEY", OWM_SECRET),
            ("OWM_LATITUDE", "39.85"),
            ("OWM_LONGITUDE", "-75.78"),
            ("OPENROUTER_API_KEY", OR_SECRET),
        ]);
        if let Some(v) = value {
            vars.insert("OWM_ENABLED", v.to_string());
            vars.insert("OPENROUTER_ENABLED", v.to_string());
        }
        let (config, _) = load(&vars).unwrap();
        let owm = config.openweathermap.unwrap().enabled;
        let or = config.openrouter.unwrap().enabled;
        assert_eq!(owm, or);
        owm
    };
    for on in ["true", "TRUE", "1", "yes", "Yes"] {
        assert!(enabled(Some(on)), "{on}");
    }
    for off in ["false", "0", "no", "off", ""] {
        assert!(!enabled(Some(off)), "{off:?}");
    }
    assert!(enabled(None), "unset = on");
}

// @spec DEPLOY-LOG-003, DEPLOY-CONFIG-010
#[test]
fn no_credential_appears_in_the_configurations_debug_output() {
    let vars = with(&[
        ("HA_TOKEN", HA_SECRET),
        ("OWM_API_KEY", OWM_SECRET),
        ("OPENROUTER_API_KEY", OR_SECRET),
        ("MCP_TOKEN", MCP_SECRET),
    ]);
    let (config, _) = load(&vars).unwrap();
    let printed = format!("{config:?}");
    for secret in [DB_SECRET, HA_SECRET, OWM_SECRET, OR_SECRET, MCP_SECRET] {
        assert!(!printed.contains(secret), "{secret} leaked");
    }
}

// @spec UI-SRV-009
#[test]
fn an_invalid_cors_origin_stops_startup_without_a_panic() {
    for bad in [
        "lawn.example.com",
        "https://lawn.example.com/app",
        "ftp://lawn.example.com",
        "http://",
    ] {
        match load(&with(&[("CORS_ALLOWED_ORIGIN", bad)])) {
            Err(TurfOpsError::Config(msg)) => assert!(msg.contains("CORS_ALLOWED_ORIGIN"), "{msg}"),
            other => panic!(
                "{bad}: expected a configuration error, got ok={}",
                other.is_ok()
            ),
        }
    }
    for good in [
        "*",
        "http://localhost:5173",
        "https://lawn.example.com",
        "  ",
    ] {
        assert!(
            load(&with(&[("CORS_ALLOWED_ORIGIN", good)])).is_ok(),
            "{good}"
        );
    }
}

// @spec WX-SRC-CFG-003
#[test]
fn a_forecast_key_without_coordinates_turns_the_forecast_off() {
    for missing in ["OWM_LATITUDE", "OWM_LONGITUDE"] {
        let mut vars = with(&[
            ("OWM_API_KEY", OWM_SECRET),
            ("OWM_LATITUDE", "39.85"),
            ("OWM_LONGITUDE", "-75.78"),
        ]);
        vars.remove(missing);
        let (config, warnings) = load(&vars).unwrap();
        assert!(
            config.openweathermap.is_none(),
            "{missing} unset: no forecast for 0, 0"
        );
        assert!(warnings.iter().any(|w| w.contains(missing)), "{warnings:?}");
    }
    let (config, _) = load(&with(&[
        ("OWM_API_KEY", OWM_SECRET),
        ("OWM_LATITUDE", "39.85"),
        ("OWM_LONGITUDE", "-75.78"),
    ]))
    .unwrap();
    let owm = config.openweathermap.unwrap();
    assert_eq!((owm.latitude, owm.longitude), (39.85, -75.78));
}

// @spec WX-SRC-CFG-003, DEPLOY-CONFIG-012
#[test]
fn an_unusable_coordinate_turns_the_forecast_off_too() {
    let coords = |lat: &str, lon: &str| {
        load(&with(&[
            ("OWM_API_KEY", OWM_SECRET),
            ("OWM_LATITUDE", lat),
            ("OWM_LONGITUDE", lon),
        ]))
        .unwrap()
    };
    for (lat, lon, named) in [
        ("39.8N", "-75.78", "OWM_LATITUDE"),
        ("39.85", "west", "OWM_LONGITUDE"),
        ("91", "-75.78", "OWM_LATITUDE"),
        ("-90.5", "-75.78", "OWM_LATITUDE"),
        ("39.85", "180.1", "OWM_LONGITUDE"),
        ("39.85", "NaN", "OWM_LONGITUDE"),
    ] {
        let (config, warnings) = coords(lat, lon);
        assert!(
            config.openweathermap.is_none(),
            "{lat}, {lon}: forecast off"
        );
        assert!(
            warnings
                .iter()
                .any(|w| w.contains(named) && w.contains("forecast is off")),
            "{lat}, {lon}: {warnings:?}"
        );
        assert!(
            !warnings.iter().any(|w| w.contains("using 0")),
            "no 0 substitute: {warnings:?}"
        );
    }
    let (config, warnings) = coords("0", "0");
    assert!(config.openweathermap.is_some(), "0, 0 is a real place");
    assert!(warnings.is_empty(), "{warnings:?}");
    let (config, _) = coords("-90", "180");
    assert!(config.openweathermap.is_some(), "the ranges are inclusive");
}
