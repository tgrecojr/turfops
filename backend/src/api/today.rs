//! `GET /api/v1/today`: the lawn's local date, so pages and forms judge "today" by the
//! server's clock rather than the viewer's.

use crate::clock::Clock;
use crate::state::AppState;
use axum::extract::State;
use axum::Json;
use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct TodayResponse {
    pub today: NaiveDate,
    /// IANA zone name; `None` when `TZ` is unset.
    pub timezone: Option<String>,
}

// @spec DEPLOY-TZ-006
pub fn today_at(clock: &Clock, now: DateTime<Utc>) -> TodayResponse {
    TodayResponse {
        today: clock.local_date(now),
        timezone: match clock {
            Clock::Zone(tz) | Clock::Frozen { zone: tz, .. } => Some(tz.name().to_string()),
            Clock::System => None,
        },
    }
}

pub async fn get_today(State(state): State<AppState>) -> Json<TodayResponse> {
    Json(today_at(&state.clock, state.clock.now()))
}

#[cfg(test)]
mod tests {
    use super::*;

    // @spec DEPLOY-TZ-006
    #[test]
    fn today_is_the_lawns_date_with_its_zone() {
        let eastern = Clock::Zone(chrono_tz::America::New_York);
        // 10:30 pm Eastern on Oct 11 is already Oct 12 in UTC.
        let now: DateTime<Utc> = "2026-10-12T02:30:00Z".parse().unwrap();
        let json = serde_json::to_value(today_at(&eastern, now)).unwrap();
        assert_eq!(json["today"], "2026-10-11");
        assert_eq!(json["timezone"], "America/New_York");
        let system = serde_json::to_value(today_at(&Clock::System, now)).unwrap();
        assert!(system["timezone"].is_null());
    }
}
