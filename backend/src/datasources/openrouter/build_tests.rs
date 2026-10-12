//! When the shared OpenRouter client exists.

use super::*;
use crate::config::load::from_vars;

fn config_with(pairs: &[(&str, &str)]) -> Option<OpenRouterConfig> {
    let vars: std::collections::HashMap<&str, &str> = [("DATABASE_PASSWORD", "x")]
        .into_iter()
        .chain(pairs.iter().copied())
        .collect();
    let (config, _) = from_vars(&|key| vars.get(key).map(|v| v.to_string())).unwrap();
    config.openrouter
}

// @spec PLANT-AI-001
#[test]
fn the_client_is_built_whenever_a_key_is_set_even_turned_off() {
    assert!(OpenRouterClient::from_config(config_with(&[]).as_ref()).is_none());
    assert!(
        OpenRouterClient::from_config(config_with(&[("OPENROUTER_API_KEY", "  ")]).as_ref())
            .is_none(),
        "a blank key is no key"
    );
    let on = OpenRouterClient::from_config(config_with(&[("OPENROUTER_API_KEY", "k")]).as_ref())
        .expect("a key builds the client");
    assert!(on.enabled(), "unset flag is on");
    for off in ["false", "0", "no", "off"] {
        let client = OpenRouterClient::from_config(
            config_with(&[("OPENROUTER_API_KEY", "k"), ("OPENROUTER_ENABLED", off)]).as_ref(),
        )
        .expect("a turned-off assistant still has a client");
        assert!(!client.enabled(), "{off}");
    }
    for on in ["true", "TRUE", "1", "yes"] {
        let client = OpenRouterClient::from_config(
            config_with(&[("OPENROUTER_API_KEY", "k"), ("OPENROUTER_ENABLED", on)]).as_ref(),
        )
        .unwrap();
        assert!(client.enabled(), "{on}");
    }
}

// @spec PLANT-AI-001
#[test]
fn startup_builds_the_shared_client_from_the_config_alone() {
    let main = include_str!("../../main.rs");
    assert!(
        main.contains("OpenRouterClient::from_config(config.openrouter.as_ref())"),
        "main.rs builds the client through from_config, with no extra conditions"
    );
}
