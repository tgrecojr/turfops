//! Pure reshaping of REST responses into model-sized tool results: chart series and
//! other bulk the model does not need are dropped, the answers are kept.

use serde_json::{json, Map, Value};

/// Drop the 3-hourly forecast from an `EnvironmentalSummary`; the daily summaries stay.
pub fn drop_hourly_forecast(summary: &mut Value) {
    if let Some(forecast) = summary.get_mut("forecast").and_then(Value::as_object_mut) {
        forecast.remove("hourly");
    }
}

/// A `TimingResponse` without the daily soil chart series.
pub fn timing_without_series(mut timing: Value) -> Value {
    if let Some(obj) = timing.as_object_mut() {
        obj.remove("series");
    }
    timing
}

/// A `DiseaseRiskResponse` reduced to one line per disease: the tier and what to do.
pub fn disease_headlines(risk: &Value) -> Value {
    let diseases: Vec<Value> = risk
        .get("diseases")
        .and_then(Value::as_array)
        .map(|list| list.iter().map(disease_headline).collect())
        .unwrap_or_default();
    json!({
        "today": risk.get("today"),
        "diseases": diseases,
        "data_notes": risk.get("data_notes"),
    })
}

fn disease_headline(disease: &Value) -> Value {
    let management = disease.get("management");
    let pick = |v: Option<&Value>, key: &str| v.and_then(|v| v.get(key)).cloned();
    json!({
        "slug": disease.get("slug"),
        "name": disease.get("name"),
        "tier": disease.get("tier"),
        "tier_note": disease.get("tier_note"),
        "score_label": disease.get("score_label"),
        "as_of": disease.get("as_of"),
        "action": pick(management, "action"),
        "headline": pick(management, "headline"),
        "protected_through": pick(management.and_then(|m| m.get("protection")), "protected_through"),
    })
}

/// A `NitrogenBudget` without its per-application list.
pub fn nitrogen_totals(mut budget: Value) -> Value {
    if let Some(obj) = budget.as_object_mut() {
        obj.remove("applications");
    }
    budget
}

/// One part of a composite result: the value, or `null` plus a note saying what is missing.
pub fn part(result: Result<Value, String>, name: &str, notes: &mut Vec<String>) -> Value {
    result.unwrap_or_else(|err| {
        notes.push(format!("{name} unavailable: {err}"));
        Value::Null
    })
}

/// Build an object from named parts.
pub fn object(parts: Vec<(&str, Value)>) -> Value {
    Value::Object(
        parts
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect::<Map<_, _>>(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_drops_only_the_hourly_forecast() {
        let mut summary = json!({
            "soil_temp_7day_avg_f": 68.2,
            "forecast": {"hourly": [1, 2, 3], "daily_summary": [{"date": "2026-09-22"}]}
        });
        drop_hourly_forecast(&mut summary);
        assert!(summary["forecast"].get("hourly").is_none());
        assert_eq!(
            summary["forecast"]["daily_summary"][0]["date"],
            "2026-09-22"
        );
        assert_eq!(summary["soil_temp_7day_avg_f"], 68.2);
    }

    #[test]
    fn summary_without_forecast_is_untouched() {
        let mut summary = json!({"soil_temp_7day_avg_f": 60.0});
        drop_hourly_forecast(&mut summary);
        assert_eq!(summary, json!({"soil_temp_7day_avg_f": 60.0}));
    }

    #[test]
    fn timing_drops_series_keeps_windows() {
        let trimmed = timing_without_series(json!({
            "windows": [{"id": "FallSeeding", "state": "Ideal"}],
            "series": [{"date": "2026-09-01"}],
            "data_notes": []
        }));
        assert!(trimmed.get("series").is_none());
        assert_eq!(trimmed["windows"][0]["state"], "Ideal");
    }

    #[test]
    fn disease_headlines_keep_tier_action_and_protection() {
        let risk = json!({
            "today": "2026-09-22",
            "station": "PA Avondale",
            "data_notes": ["lake lags a day"],
            "diseases": [{
                "slug": "brown_patch", "name": "Brown patch", "tier": "High",
                "tier_note": null, "score": 5.4, "score_label": "E-index 5.4",
                "as_of": "2026-09-21", "daily": [{"date": "2026-09-20"}],
                "methodology": {"model": "Fidanza"},
                "management": {
                    "action": "Protected", "headline": "Covered by azoxystrobin",
                    "cultural": ["water early"],
                    "protection": {"protected_through": "2026-10-01"}
                }
            }]
        });
        let trimmed = disease_headlines(&risk);
        let d = &trimmed["diseases"][0];
        assert_eq!(d["slug"], "brown_patch");
        assert_eq!(d["tier"], "High");
        assert_eq!(d["action"], "Protected");
        assert_eq!(d["protected_through"], "2026-10-01");
        assert!(d.get("daily").is_none());
        assert!(d.get("methodology").is_none());
        assert!(d.get("cultural").is_none());
        assert_eq!(trimmed["data_notes"][0], "lake lags a day");
    }

    #[test]
    fn disease_headline_without_protection_is_null() {
        let trimmed = disease_headlines(&json!({
            "diseases": [{"slug": "dollar_spot", "management": {"action": "Monitor", "protection": null}}]
        }));
        assert_eq!(trimmed["diseases"][0]["protected_through"], Value::Null);
    }

    #[test]
    fn nitrogen_totals_drop_the_application_list() {
        let trimmed = nitrogen_totals(json!({
            "year": 2026, "remaining_lbs_per_1000sqft": 1.5, "applications": [{}]
        }));
        assert!(trimmed.get("applications").is_none());
        assert_eq!(trimmed["remaining_lbs_per_1000sqft"], 1.5);
    }

    #[test]
    fn failed_part_is_null_with_a_note() {
        let mut notes = Vec::new();
        let ok = part(Ok(json!(1)), "timing", &mut notes);
        let failed = part(Err("lake down".into()), "disease_risk", &mut notes);
        assert_eq!(ok, json!(1));
        assert_eq!(failed, Value::Null);
        assert_eq!(notes, vec!["disease_risk unavailable: lake down"]);
    }
}
