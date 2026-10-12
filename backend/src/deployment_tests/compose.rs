//! The example Compose stack, checked as text.

use super::COMPOSE;

/// The lines of service `name` (indented under it), comments dropped.
fn service(name: &str) -> Vec<&'static str> {
    let header = format!("  {name}:");
    COMPOSE
        .lines()
        .skip_while(|l| *l != header)
        .skip(1)
        .take_while(|l| l.is_empty() || l.starts_with("    "))
        .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
        .collect()
}

fn has(lines: &[&str], line: &str) -> bool {
    lines.iter().any(|l| l.trim() == line)
}

/// `KEY: value` pairs of the app service's `environment:` block.
fn app_environment() -> Vec<(&'static str, &'static str)> {
    service("app")
        .into_iter()
        .skip_while(|l| l.trim() != "environment:")
        .skip(1)
        .take_while(|l| l.starts_with("      "))
        .filter_map(|l| l.trim().split_once(": "))
        .collect()
}

// @spec DEPLOY-COMPOSE-001
#[test]
fn the_stack_is_postgres_and_the_app_both_restarting() {
    let db = service("db");
    let app = service("app");
    assert!(
        db.iter()
            .any(|l| l.trim().starts_with("image: postgres:18-alpine@sha256:")),
        "postgres:18-alpine pinned by digest"
    );
    assert!(has(&db, "POSTGRES_DB: turfops") && has(&db, "POSTGRES_USER: turfops"));
    assert!(has(&db, "- turfops_data:/var/lib/postgresql"));
    assert!(COMPOSE.ends_with("volumes:\n  turfops_data:\n"));
    assert!(has(&app, "build:") && has(&app, "dockerfile: Dockerfile"));
    for lines in [&db, &app] {
        assert!(has(lines, "restart: unless-stopped"));
    }
    let services: Vec<&str> = COMPOSE
        .lines()
        .skip(1)
        .take_while(|l| !l.starts_with("volumes:"))
        .filter(|l| l.starts_with("  ") && !l.starts_with("   ") && l.ends_with(':'))
        .collect();
    assert_eq!(services, ["  db:", "  app:"]);
}

// @spec DEPLOY-COMPOSE-002
#[test]
fn one_required_password_feeds_both_services_in_the_bare_form() {
    assert!(has(&service("db"), "POSTGRES_PASSWORD: ${DB_PASSWORD:?}"));
    assert!(app_environment().contains(&("DATABASE_PASSWORD", "${DB_PASSWORD:?}")));
    assert_eq!(COMPOSE.matches("${DB_PASSWORD").count(), 2);
    assert_eq!(
        COMPOSE.matches("${DB_PASSWORD:?}").count(),
        2,
        "no message form, no default"
    );
}

// @spec DEPLOY-COMPOSE-003
#[test]
fn the_app_waits_for_a_healthy_database() {
    let app = service("app").join("\n");
    assert!(app.contains("depends_on:\n      db:\n        condition: service_healthy"));
    let db = service("db").join("\n");
    assert!(db.contains(r#"test: ["CMD-SHELL", "pg_isready -U turfops"]"#));
    assert!(db.contains("interval: 5s") && db.contains("retries: 5"));
}

// @spec DEPLOY-COMPOSE-004
#[test]
fn the_lake_is_mounted_read_only_at_data() {
    assert!(has(
        &service("app"),
        "- ${DATALAKE_HOST_PATH:-/data}:/data:ro"
    ));
}

// @spec DEPLOY-COMPOSE-005
#[test]
fn the_app_gets_a_timezone_defaulting_to_new_york() {
    assert!(app_environment().contains(&("TZ", "${TZ:-America/New_York}")));
}

// @spec DEPLOY-COMPOSE-006
#[test]
fn every_optional_setting_passes_through_empty_or_defaulted() {
    let env = app_environment();
    assert!(env.len() > 20, "{env:?}");
    for (key, value) in env {
        match key {
            "SERVER_HOST" => assert_eq!(value, r#""0.0.0.0""#),
            "SERVER_PORT" => assert_eq!(value, r#""3000""#),
            "STATIC_DIR" => assert_eq!(value, "/app/static"),
            "DATABASE_PASSWORD" => assert_eq!(value, "${DB_PASSWORD:?}"),
            "RUST_LOG" => assert_eq!(value, "${RUST_LOG:-info}"),
            _ => assert!(
                value.starts_with(&format!("${{{key}:-")) && value.ends_with('}'),
                "{key}: {value}"
            ),
        }
    }
}
