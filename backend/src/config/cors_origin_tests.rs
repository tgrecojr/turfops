//! `CORS_ALLOWED_ORIGIN` as the server reads it.

use super::load::from_vars;

/// The origin the configuration hands the CORS layer for `value`.
fn origin_for(value: Option<&str>) -> Option<String> {
    let lookup = |key: &str| match key {
        "DATABASE_PASSWORD" => Some("secret".to_string()),
        "CORS_ALLOWED_ORIGIN" => value.map(str::to_string),
        _ => None,
    };
    let (config, _) = from_vars(&lookup).unwrap();
    config.server.cors_allowed_origin
}

// @spec UI-SRV-004
#[test]
fn a_blank_origin_is_the_same_as_none() {
    for value in [None, Some(""), Some("   "), Some("\t")] {
        assert_eq!(origin_for(value), None, "{value:?}");
    }
    assert_eq!(
        origin_for(Some("https://lawn.example.com")).as_deref(),
        Some("https://lawn.example.com")
    );
    assert_eq!(origin_for(Some("*")).as_deref(), Some("*"));
}
