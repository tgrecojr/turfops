//! Reading stored rows the code no longer recognises: the read falls back and warns.

use super::{ApplicationRow, LawnProfileRow};
use crate::models::{ApplicationType, FracClass, GrassType};
use chrono::{NaiveDate, Utc};
use std::io::Write;
use std::sync::{Arc, Mutex};

/// Log output captured while `f` runs.
#[derive(Clone, Default)]
struct Captured(Arc<Mutex<Vec<u8>>>);

impl Write for Captured {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn logged<T>(f: impl FnOnce() -> T) -> (T, String) {
    let captured = Captured::default();
    let writer = captured.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_writer(move || writer.clone())
        .finish();
    crate::test_support::keep_tracing_interest_shared();
    let value = tracing::subscriber::with_default(subscriber, f);
    let log = String::from_utf8(captured.0.lock().unwrap().clone()).unwrap();
    (value, log)
}

fn profile_row(grass: &str, soil: &str, irrigation: &str) -> LawnProfileRow {
    LawnProfileRow {
        id: 1,
        name: "Front".into(),
        grass_type: grass.into(),
        usda_zone: "7a".into(),
        soil_type: Some(soil.into()),
        lawn_size_sqft: None,
        irrigation_type: Some(irrigation.into()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

// @spec APP-PROF-007
#[test]
fn an_unreadable_profile_type_falls_back_with_a_warning() {
    let (profile, log) = logged(|| profile_row("Bluegras", "Mud", "Drip").into_lawn_profile());
    assert_eq!(profile.grass_type, GrassType::TallFescue);
    assert_eq!(profile.soil_type, None);
    assert_eq!(profile.irrigation_type, None);
    for value in ["Bluegras", "Mud", "Drip"] {
        assert!(
            log.contains("WARN") && log.contains(value),
            "{value}: {log}"
        );
    }

    let (fine, quiet) = logged(|| profile_row("TallFescue", "Clay", "Hose").into_lawn_profile());
    assert!(fine.soil_type.is_some() && fine.irrigation_type.is_some());
    assert!(quiet.is_empty(), "{quiet}");
}

fn application_row(kind: &str, frac: Vec<&str>) -> ApplicationRow {
    ApplicationRow {
        id: 7,
        lawn_profile_id: 1,
        application_type: kind.into(),
        product_name: Some("Heritage G".into()),
        application_date: NaiveDate::from_ymd_opt(2026, 5, 1).unwrap(),
        rate_per_1000sqft: None,
        coverage_sqft: None,
        notes: None,
        nitrogen_pct: None,
        phosphorus_pct: None,
        potassium_pct: None,
        plant_id: None,
        plant_name: None,
        follow_up_date: None,
        frac_classes: Some(frac.into_iter().map(String::from).collect()),
        product_id: None,
        created_at: Utc::now(),
    }
}

// @spec APP-LOG-008
#[test]
fn an_unreadable_type_or_frac_code_never_fails_the_read() {
    let (app, log) =
        logged(|| application_row("Spraying", vec!["Frac11", "Frac99"]).into_application());
    assert_eq!(app.application_type, ApplicationType::Other);
    assert_eq!(
        app.frac_classes,
        Some(vec![FracClass::Frac11]),
        "the bad code is dropped"
    );
    for value in ["Spraying", "Frac99"] {
        assert!(
            log.contains("WARN") && log.contains(value),
            "{value}: {log}"
        );
    }
}
