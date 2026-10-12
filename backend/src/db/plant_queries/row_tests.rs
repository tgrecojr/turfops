//! Reading a stored plant row.

use super::*;
use crate::models::plant::{IdentificationConfidence, PlantMaintenancePlan};

fn plan() -> PlantMaintenancePlan {
    PlantMaintenancePlan {
        identified_name: "Holly".into(),
        scientific_name: None,
        identification_confidence: IdentificationConfidence::High,
        summary: String::new(),
        tasks: vec![],
        warnings: vec![],
    }
}

/// Everything logged at warn or above while `f` runs.
fn warnings_during<T>(f: impl FnOnce() -> T) -> (T, String) {
    use std::sync::{Arc, Mutex};
    let buf = Arc::new(Mutex::new(Vec::<u8>::new()));
    let sink = buf.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::WARN)
        .with_ansi(false)
        .with_writer(move || SinkWriter(sink.clone()))
        .finish();
    let out = tracing::subscriber::with_default(subscriber, f);
    let text = String::from_utf8(buf.lock().unwrap().clone()).unwrap();
    (out, text)
}

struct SinkWriter(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

impl std::io::Write for SinkWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

// @spec PLANT-DATA-003
#[test]
fn an_unknown_stored_plant_type_reads_as_other_with_a_warning() {
    let row = PlantRow {
        id: 5,
        lawn_profile_id: 1,
        common_name: "Holly".into(),
        scientific_name: None,
        plant_type: "Bush".into(),
        location: None,
        planting_date: None,
        notes: None,
        maintenance_plan: serde_json::to_value(plan()).unwrap(),
        plan_generated_at: Utc::now(),
        plan_model: "m".into(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let (read, logged) = warnings_during(|| row.into_plant());
    assert_eq!(read.unwrap().plant_type, PlantType::Other);
    assert!(logged.contains("Unknown plant_type"), "{logged}");
    assert!(logged.contains("Bush"), "{logged}");
}
