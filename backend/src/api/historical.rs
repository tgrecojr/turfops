use crate::clock::Clock;
use crate::error::{Source, TurfOpsError};
use crate::logic::gdd;
use crate::models::{EnvironmentalReading, HistoricalData, TimeSeriesPoint};
use crate::state::AppState;
use axum::extract::{Query, State};
use axum::Json;
use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct HistoricalQuery {
    pub range: Option<String>,
}

/// GET /api/v1/historical?range=7d
/// Returns time-series environmental data for the requested range, read directly from
/// the data lake (silver hourly observations + gold daily GDD).
/// Supported ranges: 7d, 30d, 90d
// @spec WX-VIEW-API-002, WX-VIEW-API-004, WX-VIEW-API-006
pub async fn get_historical(
    State(state): State<AppState>,
    Query(params): Query<HistoricalQuery>,
) -> Result<Json<HistoricalData>, TurfOpsError> {
    let range_str = params.range.as_deref().unwrap_or("7d");
    let now = Utc::now();

    // Silver is hourly; longer ranges group hours into buckets to keep point counts
    // reasonable without dropping any hour's rain.
    let (start, bucket_hours) = match range_str {
        "7d" => (now - Duration::days(7), 1),    // ~168 hourly points
        "30d" => (now - Duration::days(30), 6),  // 6-hour buckets
        "90d" => (now - Duration::days(90), 24), // 24-hour buckets
        _ => {
            return Err(TurfOpsError::InvalidData(
                "Invalid range. Use 7d, 30d, or 90d".into(),
            ));
        }
    };

    let service = state.sync_service.read().await;
    let client = service
        .weather_client()
        .ok_or_else(|| TurfOpsError::not_configured(Source::WeatherLake))?;

    // fetch_range returns newest-first; reverse to ascending for charting.
    let mut readings = client.fetch_range(start, now).await?;
    readings.reverse();

    let Series {
        soil_temp_10_f,
        ambient_temp_f,
        humidity_percent,
        soil_moisture_10,
        precipitation_mm,
    } = series(&readings, bucket_hours);

    // GDD accumulation from the gold daily layer. Accumulate from Jan 1 so the values
    // are true year-to-date (resetting on year boundaries), then keep only points
    // within the requested range.
    let (accum_start, start_date, end_date) = gdd_window(&state.clock, start, now);
    let gdd_records =
        gdd::accumulate_daily_gdd(&client.fetch_daily_gdd(accum_start, end_date).await?);

    let gdd_accumulation: Vec<TimeSeriesPoint> = gdd_records
        .iter()
        .filter(|d| d.date >= start_date)
        .map(|d| TimeSeriesPoint {
            timestamp: d.date.and_hms_opt(12, 0, 0).unwrap().and_utc(),
            value: d.cumulative_gdd_base50,
        })
        .collect();

    Ok(Json(HistoricalData {
        range: range_str.to_string(),
        soil_temp_10_f,
        ambient_temp_f,
        humidity_percent,
        soil_moisture_10,
        precipitation_mm,
        gdd_accumulation,
    }))
}

/// The chart series for a range's hourly readings (ascending).
pub(crate) struct Series {
    pub soil_temp_10_f: Vec<TimeSeriesPoint>,
    pub ambient_temp_f: Vec<TimeSeriesPoint>,
    pub humidity_percent: Vec<TimeSeriesPoint>,
    pub soil_moisture_10: Vec<TimeSeriesPoint>,
    pub precipitation_mm: Vec<TimeSeriesPoint>,
}

/// One series' values in a bucket: rain sums, everything else averages over the hours
/// that reported it.
#[derive(Default)]
struct Bucket {
    sum: f64,
    count: u32,
}

impl Bucket {
    fn add(&mut self, value: Option<f64>) {
        if let Some(v) = value {
            self.sum += v;
            self.count += 1;
        }
    }
}

/// Group the hours into `bucket_hours`-long time buckets (1 = every hour), each point
/// stamped at its bucket's start. A bucket with no value for a series is left out of it.
// @spec WX-VIEW-API-003
pub(crate) fn series(readings: &[EnvironmentalReading], bucket_hours: i64) -> Series {
    let bucket_secs = bucket_hours.max(1) * 3600;
    let mut buckets: Vec<(DateTime<Utc>, [Bucket; 5])> = Vec::new();
    for r in readings {
        let start_secs = r.timestamp.timestamp().div_euclid(bucket_secs) * bucket_secs;
        let start = if bucket_hours <= 1 {
            r.timestamp
        } else {
            DateTime::from_timestamp(start_secs, 0).unwrap_or(r.timestamp)
        };
        if buckets.last().is_none_or(|(s, _)| *s != start) {
            buckets.push((start, Default::default()));
        }
        let [soil, ambient, humidity, moisture, rain] = &mut buckets.last_mut().unwrap().1;
        soil.add(r.soil_temp_10_f);
        ambient.add(r.ambient_temp_f);
        humidity.add(r.humidity_percent);
        moisture.add(r.soil_moisture_10);
        rain.add(r.precipitation_mm.filter(|p| *p >= 0.0));
    }

    let points = |index: usize, total: bool| -> Vec<TimeSeriesPoint> {
        buckets
            .iter()
            .filter(|(_, b)| b[index].count > 0)
            .map(|(timestamp, b)| TimeSeriesPoint {
                timestamp: *timestamp,
                value: if total {
                    b[index].sum
                } else {
                    b[index].sum / f64::from(b[index].count)
                },
            })
            .collect()
    };
    Series {
        soil_temp_10_f: points(0, false),
        ambient_temp_f: points(1, false),
        humidity_percent: points(2, false),
        soil_moisture_10: points(3, false),
        precipitation_mm: points(4, true),
    }
}

/// The GDD series' dates for a range from `start` to `now`, as the lawn's local dates:
/// (Jan 1 of the start's year to accumulate from, the first date shown, the last date).
// @spec WX-VIEW-API-005
fn gdd_window(
    clock: &Clock,
    start: DateTime<Utc>,
    now: DateTime<Utc>,
) -> (NaiveDate, NaiveDate, NaiveDate) {
    let start_date = clock.local_date(start);
    let accum_start = NaiveDate::from_ymd_opt(start_date.year(), 1, 1).unwrap_or(start_date);
    (accum_start, start_date, clock.local_date(now))
}

#[cfg(test)]
mod handler_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::DataSource;

    fn hour(h: i64, soil: f64, rain: f64) -> EnvironmentalReading {
        let mut r = EnvironmentalReading::new(DataSource::SoilData);
        r.timestamp = "2026-10-01T00:00:00Z".parse::<DateTime<Utc>>().unwrap() + Duration::hours(h);
        r.soil_temp_10_f = Some(soil);
        r.ambient_temp_f = Some(soil + 10.0);
        r.precipitation_mm = Some(rain);
        r
    }

    // @spec WX-VIEW-API-003
    #[test]
    fn longer_ranges_bucket_hours_summing_rain_and_averaging_the_rest() {
        let hours: Vec<_> = (0..12).map(|h| hour(h, 50.0 + h as f64, 1.0)).collect();
        let six = series(&hours, 6);
        assert_eq!(six.soil_temp_10_f.len(), 2);
        assert_eq!(six.soil_temp_10_f[0].value, 52.5, "mean of hours 0-5");
        assert_eq!(six.soil_temp_10_f[1].value, 58.5, "mean of hours 6-11");
        assert_eq!(six.precipitation_mm[0].value, 6.0, "six hours of 1 mm");
        assert_eq!(six.ambient_temp_f[0].value, 62.5);
        let mut gappy = hours.clone();
        gappy[1].soil_temp_10_f = None;
        let mean_of_present = (50.0 + 52.0 + 53.0 + 54.0 + 55.0) / 5.0;
        assert_eq!(series(&gappy, 6).soil_temp_10_f[0].value, mean_of_present);
        assert_eq!(
            series(&hours, 1).soil_temp_10_f.len(),
            12,
            "7d keeps every hour"
        );
    }

    // @spec WX-VIEW-API-005
    #[test]
    fn the_gdd_window_uses_the_lawns_dates() {
        let clock = Clock::Zone(chrono_tz::America::New_York);
        // 10:30 pm Eastern on New Year's Eve; UTC is already Jan 1.
        let now: DateTime<Utc> = "2026-01-01T03:30:00Z".parse().unwrap();
        let d = |y, m, day| NaiveDate::from_ymd_opt(y, m, day).unwrap();
        assert_eq!(
            gdd_window(&clock, now - Duration::days(7), now),
            (d(2025, 1, 1), d(2025, 12, 24), d(2025, 12, 31))
        );
    }
}
