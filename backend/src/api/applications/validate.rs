//! Checks on an application create or update request that need no database.

use super::CreateApplicationRequest;
use crate::error::TurfOpsError;
use crate::models::{Application, ApplicationScope, ApplicationType, LogScope};
use chrono::NaiveDate;
use std::str::FromStr;

/// The checked type and dates of a create or update request.
pub(super) struct Validated {
    pub application_type: ApplicationType,
    pub application_date: NaiveDate,
    pub follow_up_date: Option<NaiveDate>,
}

/// Check a create or update request's type, dates and plant link.
// @spec APP-API-009, APP-API-010, APP-LOG-012
pub(super) fn validate(
    req: &CreateApplicationRequest,
    existing: Option<&Application>,
) -> Result<Validated, TurfOpsError> {
    let application_type = ApplicationType::from_str(&req.application_type).map_err(|_| {
        TurfOpsError::InvalidData(format!(
            "Unknown application type: {}",
            req.application_type
        ))
    })?;

    let application_date =
        NaiveDate::parse_from_str(&req.application_date, "%Y-%m-%d").map_err(|_| {
            TurfOpsError::InvalidData(format!(
                "The date \"{}\" isn't a valid date. Pick it from the calendar.",
                req.application_date
            ))
        })?;

    // An entry whose plant was deleted keeps its type without a plant (APP-LOG-012).
    let deleted_plants_entry = existing.is_some_and(|e| {
        e.plant_id.is_none() && e.plant_name.is_some() && e.application_type == application_type
    });
    match application_type.scope() {
        ApplicationScope::PlantRequired if req.plant_id.is_none() && !deleted_plants_entry => {
            return Err(TurfOpsError::InvalidData(format!(
                "{} is logged against a landscape plant. Pick the plant.",
                application_type
            )));
        }
        ApplicationScope::TurfOnly if req.plant_id.is_some() => {
            return Err(TurfOpsError::InvalidData(format!(
                "{} is a lawn application and can't be linked to a plant.",
                application_type
            )));
        }
        _ => {}
    }

    check_amounts(req)?;

    let follow_up_date = req
        .follow_up_date
        .as_deref()
        .map(|s| {
            NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|_| {
                TurfOpsError::InvalidData(format!(
                    "The follow-up date \"{}\" isn't a valid date. Pick it from the calendar.",
                    s
                ))
            })
        })
        .transpose()?;

    if let Some(fu) = follow_up_date {
        if fu < application_date {
            return Err(TurfOpsError::InvalidData(
                "The follow-up date must be on or after the application date.".into(),
            ));
        }
    }

    Ok(Validated {
        application_type,
        application_date,
        follow_up_date,
    })
}

pub(super) fn parse_scope(scope: Option<&str>) -> Result<Option<LogScope>, TurfOpsError> {
    match scope {
        None => Ok(None),
        Some("turf") => Ok(Some(LogScope::Turf)),
        Some("landscape") => Ok(Some(LogScope::Landscape)),
        Some(other) => Err(TurfOpsError::InvalidData(format!(
            "Unknown scope \"{other}\": use turf or landscape."
        ))),
    }
}

/// The stored entry an update produces: every user field from the request.
/// The deleted-plant marker is the system's, not the owner's, so it carries over.
pub(super) fn replace_user_fields(existing: &Application, incoming: Application) -> Application {
    Application {
        lawn_profile_id: existing.lawn_profile_id,
        created_at: existing.created_at,
        plant_name: existing.plant_name.clone(),
        ..incoming
    }
}

/// The follow-up reminder whose saved answer an update must clear, if any.
pub(super) fn follow_up_answer_to_clear(
    id: i64,
    before: Option<NaiveDate>,
    after: Option<NaiveDate>,
) -> Option<String> {
    (before != after).then(|| format!("application_followup_{id}"))
}

/// Rates and coverage are never negative; N, P and K are at most 100 %. A date in the
/// future is a planned entry and is fine.
// @spec APP-API-010
fn check_amounts(req: &CreateApplicationRequest) -> Result<(), TurfOpsError> {
    for (value, field) in [
        (req.rate_per_1000sqft, "Rate per 1,000 sq ft"),
        (req.coverage_sqft, "Coverage"),
    ] {
        if value.is_some_and(|v| v < 0.0) {
            return Err(TurfOpsError::InvalidData(format!(
                "{field} can't be negative."
            )));
        }
    }
    for (value, field) in [
        (req.nitrogen_pct, "N"),
        (req.phosphorus_pct, "P"),
        (req.potassium_pct, "K"),
    ] {
        if value.is_some_and(|v| v > 100.0) {
            return Err(TurfOpsError::InvalidData(format!(
                "{field} is a percentage and can't be more than 100."
            )));
        }
    }
    Ok(())
}

/// 404 when a delete touched no row.
pub(super) fn deleted(rows_affected: u64, id: i64) -> Result<(), TurfOpsError> {
    if rows_affected == 0 {
        return Err(TurfOpsError::NotFound(format!(
            "Application {id} not found"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(kind: &str, date: &str) -> CreateApplicationRequest {
        CreateApplicationRequest {
            application_type: kind.into(),
            product_name: None,
            application_date: date.into(),
            rate_per_1000sqft: None,
            coverage_sqft: None,
            notes: None,
            nitrogen_pct: None,
            phosphorus_pct: None,
            potassium_pct: None,
            plant_id: None,
            follow_up_date: None,
            frac_classes: None,
            product_id: None,
        }
    }

    fn message(req: CreateApplicationRequest) -> String {
        match validate(&req, None) {
            Err(TurfOpsError::InvalidData(msg)) => msg,
            other => panic!("expected a validation error, got {:?}", other.is_ok()),
        }
    }

    /// Request field names the owner never sees on the form.
    fn names_no_request_field(msg: &str) {
        for field in [
            "plant_id",
            "follow_up_date",
            "application_date",
            "YYYY-MM-DD",
        ] {
            assert!(!msg.contains(field), "{field} in {msg:?}");
        }
    }

    // @spec APP-API-009
    #[test]
    fn validation_messages_use_the_forms_words() {
        names_no_request_field(&message(request("PlantFertilizer", "2026-05-01")));

        let mut bad_follow_up = request("Fertilizer", "2026-05-01");
        bad_follow_up.follow_up_date = Some("05/20/2026".into());
        names_no_request_field(&message(bad_follow_up));

        let mut early_follow_up = request("Fertilizer", "2026-05-01");
        early_follow_up.follow_up_date = Some("2026-04-20".into());
        names_no_request_field(&message(early_follow_up));

        names_no_request_field(&message(request("Fertilizer", "May 1")));
    }

    fn stored(kind: ApplicationType) -> Application {
        Application {
            id: Some(9),
            lawn_profile_id: 3,
            application_type: kind,
            product_name: Some("Old".into()),
            application_date: NaiveDate::from_ymd_opt(2026, 4, 1).unwrap(),
            rate_per_1000sqft: Some(2.0),
            coverage_sqft: Some(500.0),
            notes: Some("old note".into()),
            plant_name: None,
            nitrogen_pct: None,
            phosphorus_pct: None,
            potassium_pct: None,
            plant_id: None,
            follow_up_date: None,
            frac_classes: None,
            product_id: None,
            created_at: chrono::DateTime::from_timestamp(1_700_000_000, 0).unwrap(),
        }
    }

    // @spec APP-API-010
    #[test]
    fn negative_amounts_and_npk_over_100_are_named() {
        let mut negative_rate = request("Fertilizer", "2026-05-01");
        negative_rate.rate_per_1000sqft = Some(-1.0);
        assert!(message(negative_rate).contains("Rate"));
        let mut negative_area = request("Fertilizer", "2026-05-01");
        negative_area.coverage_sqft = Some(-50.0);
        assert!(message(negative_area).contains("Coverage"));
        let mut too_much_n = request("Fertilizer", "2026-05-01");
        too_much_n.nitrogen_pct = Some(146.0);
        let msg = message(too_much_n);
        assert!(msg.contains("N") && msg.contains("100"), "{msg}");
        let mut too_much_k = request("Fertilizer", "2026-05-01");
        too_much_k.potassium_pct = Some(101.0);
        assert!(message(too_much_k).contains("K"));
        let mut zero = request("Fertilizer", "2099-05-01");
        zero.rate_per_1000sqft = Some(0.0);
        zero.nitrogen_pct = Some(100.0);
        assert!(
            validate(&zero, None).is_ok(),
            "zero, 100 % and a future date are fine"
        );
    }

    // @spec APP-LOG-012
    #[test]
    fn a_deleted_plants_entry_stays_editable_without_a_plant() {
        let mut orphan = stored(ApplicationType::PlantFertilizer);
        orphan.plant_name = Some("Viburnum".into());
        let same_type = request("PlantFertilizer", "2026-04-02");
        assert!(validate(&same_type, Some(&orphan)).is_ok());
        let new_type = request("Mulching", "2026-04-02");
        assert!(
            validate(&new_type, Some(&orphan)).is_err(),
            "a changed type needs a plant"
        );
        let live = stored(ApplicationType::PlantFertilizer);
        assert!(
            validate(&same_type, Some(&live)).is_err(),
            "not a deleted plant's entry"
        );
    }

    // @spec APP-API-012
    #[test]
    fn scope_is_turf_landscape_or_nothing() {
        assert_eq!(parse_scope(None).unwrap(), None);
        assert_eq!(parse_scope(Some("turf")).unwrap(), Some(LogScope::Turf));
        assert_eq!(
            parse_scope(Some("landscape")).unwrap(),
            Some(LogScope::Landscape)
        );
        assert!(matches!(
            parse_scope(Some("plants")),
            Err(TurfOpsError::InvalidData(_))
        ));
    }

    // @spec APP-API-005
    #[test]
    fn an_update_replaces_every_user_field() {
        let mut existing = stored(ApplicationType::PlantFertilizer);
        existing.plant_name = Some("Viburnum".into());
        let incoming = Application {
            lawn_profile_id: 0,
            notes: None,
            coverage_sqft: None,
            plant_name: None,
            created_at: chrono::Utc::now(),
            ..stored(ApplicationType::PlantFertilizer)
        };
        let updated = replace_user_fields(&existing, incoming);
        assert_eq!(
            (updated.notes, updated.coverage_sqft),
            (None, None),
            "omitted = cleared"
        );
        assert_eq!(updated.lawn_profile_id, 3);
        assert_eq!(updated.created_at, existing.created_at);
        assert_eq!(
            updated.plant_name.as_deref(),
            Some("Viburnum"),
            "still a deleted plant's entry"
        );
    }

    // @spec APP-FOLLOW-008
    #[test]
    fn a_rescheduled_follow_up_starts_unanswered() {
        let d = |day| NaiveDate::from_ymd_opt(2026, 5, day);
        assert_eq!(
            follow_up_answer_to_clear(9, d(8), d(15)).as_deref(),
            Some("application_followup_9")
        );
        assert_eq!(
            follow_up_answer_to_clear(9, d(8), None).as_deref(),
            Some("application_followup_9"),
            "cleared"
        );
        assert_eq!(follow_up_answer_to_clear(9, d(8), d(8)), None, "unchanged");
        assert_eq!(follow_up_answer_to_clear(9, None, None), None);
    }

    // @spec APP-API-011
    #[test]
    fn deleting_an_unknown_entry_is_not_found() {
        assert!(deleted(1, 9).is_ok());
        assert!(matches!(deleted(0, 9), Err(TurfOpsError::NotFound(_))));
    }
}
