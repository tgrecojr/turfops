use crate::error::{Result, TurfOpsError};
use crate::models::{
    seasonal_plan::ThresholdCrossing, Application, ApplicationType, GrassType, IrrigationType,
    LawnProfile, SoilType, WeatherSnapshot,
};
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgPool;
use std::str::FromStr;
use tracing::warn;

/// Safely convert a Serialize enum variant to its string representation for DB storage.
fn enum_to_db_string<T: serde::Serialize>(value: T) -> Result<String> {
    let json_val = serde_json::to_value(value)
        .map_err(|e| TurfOpsError::InvalidData(format!("Failed to serialize enum: {}", e)))?;
    json_val
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| TurfOpsError::InvalidData("Expected string enum variant".into()))
}

/// Safely convert an Option<Serialize enum> to Option<String> for DB storage.
fn opt_enum_to_db_string<T: serde::Serialize>(value: Option<T>) -> Result<Option<String>> {
    match value {
        Some(v) => Ok(Some(enum_to_db_string(v)?)),
        None => Ok(None),
    }
}

// Lawn Profile Queries

pub async fn create_lawn_profile(pool: &PgPool, profile: &LawnProfile) -> Result<i64> {
    let row = sqlx::query_scalar::<_, i64>(
        r#"
        INSERT INTO lawn_profiles
            (name, grass_type, usda_zone, soil_type, lawn_size_sqft, irrigation_type, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id
        "#,
    )
    .bind(&profile.name)
    .bind(enum_to_db_string(profile.grass_type)?)
    .bind(&profile.usda_zone)
    .bind(opt_enum_to_db_string(profile.soil_type)?)
    .bind(profile.lawn_size_sqft)
    .bind(opt_enum_to_db_string(profile.irrigation_type)?)
    .bind(profile.created_at)
    .bind(profile.updated_at)
    .fetch_one(pool)
    .await?;

    Ok(row)
}

pub async fn get_default_lawn_profile(pool: &PgPool) -> Result<Option<LawnProfile>> {
    let row = sqlx::query_as::<_, LawnProfileRow>(
        "SELECT id, name, grass_type, usda_zone, soil_type, lawn_size_sqft, irrigation_type, created_at, updated_at FROM lawn_profiles ORDER BY id LIMIT 1",
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| r.into_lawn_profile()))
}

pub async fn update_lawn_profile(pool: &PgPool, profile: &LawnProfile) -> Result<()> {
    let id = profile
        .id
        .ok_or_else(|| TurfOpsError::InvalidData("Profile has no ID".into()))?;

    sqlx::query(
        r#"
        UPDATE lawn_profiles SET
            name = $1, grass_type = $2, usda_zone = $3, soil_type = $4,
            lawn_size_sqft = $5, irrigation_type = $6, updated_at = $7
        WHERE id = $8
        "#,
    )
    .bind(&profile.name)
    .bind(enum_to_db_string(profile.grass_type)?)
    .bind(&profile.usda_zone)
    .bind(opt_enum_to_db_string(profile.soil_type)?)
    .bind(profile.lawn_size_sqft)
    .bind(opt_enum_to_db_string(profile.irrigation_type)?)
    .bind(Utc::now())
    .bind(id)
    .execute(pool)
    .await?;

    Ok(())
}

// Application Queries

pub async fn get_applications_for_profile(
    pool: &PgPool,
    profile_id: i64,
    limit: i64,
    offset: i64,
) -> Result<Vec<Application>> {
    let rows = sqlx::query_as::<_, ApplicationRow>(
        r#"SELECT id, lawn_profile_id, application_type, product_name, application_date,
           rate_per_1000sqft, coverage_sqft, notes, soil_temp_10cm_f, ambient_temp_f,
           humidity_percent, soil_moisture, nitrogen_pct, phosphorus_pct, potassium_pct,
           plant_id, follow_up_date, created_at
           FROM applications WHERE lawn_profile_id = $1 ORDER BY application_date DESC
           LIMIT $2 OFFSET $3"#,
    )
    .bind(profile_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| r.into_application()).collect())
}

pub async fn get_applications_for_profile_in_range(
    pool: &PgPool,
    profile_id: i64,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<Vec<Application>> {
    let rows = sqlx::query_as::<_, ApplicationRow>(
        r#"SELECT id, lawn_profile_id, application_type, product_name, application_date,
           rate_per_1000sqft, coverage_sqft, notes, soil_temp_10cm_f, ambient_temp_f,
           humidity_percent, soil_moisture, nitrogen_pct, phosphorus_pct, potassium_pct,
           plant_id, follow_up_date, created_at
           FROM applications
           WHERE lawn_profile_id = $1
             AND (
               (application_date >= $2 AND application_date < $3)
               OR (follow_up_date >= $2 AND follow_up_date < $3)
             )
           ORDER BY application_date DESC"#,
    )
    .bind(profile_id)
    .bind(start_date)
    .bind(end_date)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| r.into_application()).collect())
}

pub async fn create_application(pool: &PgPool, app: &Application) -> Result<i64> {
    let weather = &app.weather_snapshot;
    let row = sqlx::query_scalar::<_, i64>(
        r#"
        INSERT INTO applications
            (lawn_profile_id, application_type, product_name, application_date,
             rate_per_1000sqft, coverage_sqft, notes,
             soil_temp_10cm_f, ambient_temp_f, humidity_percent, soil_moisture,
             nitrogen_pct, phosphorus_pct, potassium_pct, plant_id, follow_up_date)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
        RETURNING id
        "#,
    )
    .bind(app.lawn_profile_id)
    .bind(enum_to_db_string(app.application_type)?)
    .bind(&app.product_name)
    .bind(app.application_date)
    .bind(app.rate_per_1000sqft)
    .bind(app.coverage_sqft)
    .bind(&app.notes)
    .bind(weather.as_ref().and_then(|w| w.soil_temp_10cm_f))
    .bind(weather.as_ref().and_then(|w| w.ambient_temp_f))
    .bind(weather.as_ref().and_then(|w| w.humidity_percent))
    .bind(weather.as_ref().and_then(|w| w.soil_moisture))
    .bind(app.nitrogen_pct)
    .bind(app.phosphorus_pct)
    .bind(app.potassium_pct)
    .bind(app.plant_id)
    .bind(app.follow_up_date)
    .fetch_one(pool)
    .await?;

    Ok(row)
}

pub async fn update_application(pool: &PgPool, app: &Application) -> Result<u64> {
    let id = app.id.ok_or_else(|| {
        TurfOpsError::InvalidData("update_application requires Application.id".into())
    })?;
    let weather = &app.weather_snapshot;
    let result = sqlx::query(
        r#"
        UPDATE applications
           SET application_type = $2,
               product_name = $3,
               application_date = $4,
               rate_per_1000sqft = $5,
               coverage_sqft = $6,
               notes = $7,
               soil_temp_10cm_f = $8,
               ambient_temp_f = $9,
               humidity_percent = $10,
               soil_moisture = $11,
               nitrogen_pct = $12,
               phosphorus_pct = $13,
               potassium_pct = $14,
               plant_id = $15,
               follow_up_date = $16
         WHERE id = $1
        "#,
    )
    .bind(id)
    .bind(enum_to_db_string(app.application_type)?)
    .bind(&app.product_name)
    .bind(app.application_date)
    .bind(app.rate_per_1000sqft)
    .bind(app.coverage_sqft)
    .bind(&app.notes)
    .bind(weather.as_ref().and_then(|w| w.soil_temp_10cm_f))
    .bind(weather.as_ref().and_then(|w| w.ambient_temp_f))
    .bind(weather.as_ref().and_then(|w| w.humidity_percent))
    .bind(weather.as_ref().and_then(|w| w.soil_moisture))
    .bind(app.nitrogen_pct)
    .bind(app.phosphorus_pct)
    .bind(app.potassium_pct)
    .bind(app.plant_id)
    .bind(app.follow_up_date)
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

pub async fn get_application_by_id(pool: &PgPool, id: i64) -> Result<Option<Application>> {
    let row = sqlx::query_as::<_, ApplicationRow>(
        r#"SELECT id, lawn_profile_id, application_type, product_name, application_date,
                  rate_per_1000sqft, coverage_sqft, notes, soil_temp_10cm_f, ambient_temp_f,
                  humidity_percent, soil_moisture, nitrogen_pct, phosphorus_pct, potassium_pct,
                  plant_id, follow_up_date, created_at
           FROM applications WHERE id = $1"#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| r.into_application()))
}

pub async fn delete_application(pool: &PgPool, id: i64) -> Result<()> {
    sqlx::query("DELETE FROM applications WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

// Recommendation State Queries

pub async fn get_recommendation_states(
    pool: &PgPool,
) -> Result<std::collections::HashMap<String, (bool, bool)>> {
    let rows = sqlx::query_as::<_, (String, bool, bool)>(
        "SELECT id, dismissed, addressed FROM recommendation_states",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|(id, d, a)| (id, (d, a))).collect())
}

pub async fn upsert_recommendation_state(
    pool: &PgPool,
    id: &str,
    dismissed: bool,
    addressed: bool,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO recommendation_states (id, dismissed, addressed, updated_at)
        VALUES ($1, $2, $3, NOW())
        ON CONFLICT (id) DO UPDATE SET
            dismissed = $2,
            addressed = $3,
            updated_at = NOW()
        "#,
    )
    .bind(id)
    .bind(dismissed)
    .bind(addressed)
    .execute(pool)
    .await?;

    Ok(())
}

// Row types for sqlx mapping

#[derive(sqlx::FromRow)]
struct LawnProfileRow {
    id: i64,
    name: String,
    grass_type: String,
    usda_zone: String,
    soil_type: Option<String>,
    lawn_size_sqft: Option<f64>,
    irrigation_type: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl LawnProfileRow {
    fn into_lawn_profile(self) -> LawnProfile {
        let grass_type = GrassType::from_str(&self.grass_type).unwrap_or_else(|_| {
            warn!(
                grass_type = %self.grass_type,
                "Unknown grass_type in database, defaulting to TallFescue"
            );
            GrassType::TallFescue
        });
        let soil_type = self.soil_type.as_ref().and_then(|s| {
            SoilType::from_str(s).ok().or_else(|| {
                warn!(soil_type = %s, "Unknown soil_type in database, ignoring");
                None
            })
        });
        let irrigation_type = self.irrigation_type.as_ref().and_then(|i| {
            IrrigationType::from_str(i).ok().or_else(|| {
                warn!(irrigation_type = %i, "Unknown irrigation_type in database, ignoring");
                None
            })
        });

        LawnProfile {
            id: Some(self.id),
            name: self.name,
            grass_type,
            usda_zone: self.usda_zone,
            soil_type,
            lawn_size_sqft: self.lawn_size_sqft,
            irrigation_type,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct ApplicationRow {
    id: i64,
    lawn_profile_id: i64,
    application_type: String,
    product_name: Option<String>,
    application_date: NaiveDate,
    rate_per_1000sqft: Option<f64>,
    coverage_sqft: Option<f64>,
    notes: Option<String>,
    soil_temp_10cm_f: Option<f64>,
    ambient_temp_f: Option<f64>,
    humidity_percent: Option<f64>,
    soil_moisture: Option<f64>,
    nitrogen_pct: Option<f64>,
    phosphorus_pct: Option<f64>,
    potassium_pct: Option<f64>,
    plant_id: Option<i64>,
    follow_up_date: Option<NaiveDate>,
    created_at: DateTime<Utc>,
}

impl ApplicationRow {
    fn into_application(self) -> Application {
        let weather = if self.soil_temp_10cm_f.is_some()
            || self.ambient_temp_f.is_some()
            || self.humidity_percent.is_some()
            || self.soil_moisture.is_some()
        {
            Some(WeatherSnapshot {
                soil_temp_10cm_f: self.soil_temp_10cm_f,
                ambient_temp_f: self.ambient_temp_f,
                humidity_percent: self.humidity_percent,
                soil_moisture: self.soil_moisture,
            })
        } else {
            None
        };

        let application_type =
            ApplicationType::from_str(&self.application_type).unwrap_or_else(|_| {
                warn!(
                    application_type = %self.application_type,
                    "Unknown application_type in database, defaulting to Other"
                );
                ApplicationType::Other
            });

        Application {
            id: Some(self.id),
            lawn_profile_id: self.lawn_profile_id,
            application_type,
            product_name: self.product_name,
            application_date: self.application_date,
            rate_per_1000sqft: self.rate_per_1000sqft,
            coverage_sqft: self.coverage_sqft,
            notes: self.notes,
            weather_snapshot: weather,
            nitrogen_pct: self.nitrogen_pct,
            phosphorus_pct: self.phosphorus_pct,
            potassium_pct: self.potassium_pct,
            plant_id: self.plant_id,
            follow_up_date: self.follow_up_date,
            created_at: self.created_at,
        }
    }
}

// Seasonal Plan / Threshold Crossing Cache Queries

pub async fn get_threshold_crossings(pool: &PgPool) -> Result<Vec<ThresholdCrossing>> {
    let rows = sqlx::query_as::<_, ThresholdCrossingRow>(
        r#"SELECT year, threshold_name, crossing_date, avg_soil_temp_f
           FROM seasonal_threshold_crossings
           ORDER BY year ASC, crossing_date ASC"#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| ThresholdCrossing {
            year: r.year,
            threshold_name: r.threshold_name,
            crossing_date: r.crossing_date,
            avg_soil_temp_f: r.avg_soil_temp_f,
        })
        .collect())
}

pub async fn get_threshold_crossings_years(pool: &PgPool) -> Result<Vec<i32>> {
    let rows = sqlx::query_scalar::<_, i32>(
        "SELECT DISTINCT year FROM seasonal_threshold_crossings ORDER BY year ASC",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn upsert_threshold_crossing(pool: &PgPool, crossing: &ThresholdCrossing) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO seasonal_threshold_crossings (year, threshold_name, crossing_date, avg_soil_temp_f, computed_at)
        VALUES ($1, $2, $3, $4, NOW())
        ON CONFLICT (year, threshold_name) DO UPDATE SET
            crossing_date = $3, avg_soil_temp_f = $4, computed_at = NOW()
        "#,
    )
    .bind(crossing.year)
    .bind(&crossing.threshold_name)
    .bind(crossing.crossing_date)
    .bind(crossing.avg_soil_temp_f)
    .execute(pool)
    .await?;
    Ok(())
}

#[derive(sqlx::FromRow)]
struct ThresholdCrossingRow {
    year: i32,
    threshold_name: String,
    crossing_date: NaiveDate,
    avg_soil_temp_f: f64,
}
