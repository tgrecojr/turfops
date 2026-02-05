use crate::db::Database;
use crate::error::{Result, TurfOpsError};
use crate::models::{
    Application, ApplicationType, EnvironmentalReading, GrassType, IrrigationType, LawnProfile,
    SoilType, WeatherSnapshot,
};
use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::{params, Row};
use tracing::warn;

// Lawn Profile Queries

impl Database {
    pub fn create_lawn_profile(&self, profile: &LawnProfile) -> Result<i64> {
        self.with_conn(|conn| {
            conn.execute(
                r#"
                INSERT INTO lawn_profiles
                    (name, grass_type, usda_zone, soil_type, lawn_size_sqft, irrigation_type, created_at, updated_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                "#,
                params![
                    profile.name,
                    format!("{:?}", profile.grass_type),
                    profile.usda_zone,
                    profile.soil_type.map(|s| format!("{:?}", s)),
                    profile.lawn_size_sqft,
                    profile.irrigation_type.map(|i| format!("{:?}", i)),
                    profile.created_at.to_rfc3339(),
                    profile.updated_at.to_rfc3339(),
                ],
            )?;
            Ok(conn.last_insert_rowid())
        })
    }

    pub fn get_default_lawn_profile(&self) -> Result<Option<LawnProfile>> {
        self.with_conn(|conn| {
            conn.query_row(
                "SELECT * FROM lawn_profiles ORDER BY id LIMIT 1",
                [],
                row_to_lawn_profile,
            )
            .optional()
            .map_err(Into::into)
        })
    }

    pub fn update_lawn_profile(&self, profile: &LawnProfile) -> Result<()> {
        let id = profile
            .id
            .ok_or_else(|| TurfOpsError::InvalidData("Profile has no ID".into()))?;

        self.with_conn(|conn| {
            conn.execute(
                r#"
                UPDATE lawn_profiles SET
                    name = ?1, grass_type = ?2, usda_zone = ?3, soil_type = ?4,
                    lawn_size_sqft = ?5, irrigation_type = ?6, updated_at = ?7
                WHERE id = ?8
                "#,
                params![
                    profile.name,
                    format!("{:?}", profile.grass_type),
                    profile.usda_zone,
                    profile.soil_type.map(|s| format!("{:?}", s)),
                    profile.lawn_size_sqft,
                    profile.irrigation_type.map(|i| format!("{:?}", i)),
                    Utc::now().to_rfc3339(),
                    id,
                ],
            )?;
            Ok(())
        })
    }
}

fn row_to_lawn_profile(row: &Row) -> rusqlite::Result<LawnProfile> {
    let grass_type_str: String = row.get("grass_type")?;
    let soil_type_str: Option<String> = row.get("soil_type")?;
    let irrigation_type_str: Option<String> = row.get("irrigation_type")?;
    let created_at_str: String = row.get("created_at")?;
    let updated_at_str: String = row.get("updated_at")?;

    let grass_type = GrassType::from_str(&grass_type_str).unwrap_or_else(|| {
        warn!(
            grass_type = %grass_type_str,
            "Unknown grass_type in database, defaulting to TallFescue"
        );
        GrassType::TallFescue
    });
    let soil_type = soil_type_str.as_ref().and_then(|s| {
        SoilType::from_str(s).or_else(|| {
            warn!(soil_type = %s, "Unknown soil_type in database, ignoring");
            None
        })
    });
    let irrigation_type = irrigation_type_str.as_ref().and_then(|i| {
        IrrigationType::from_str(i).or_else(|| {
            warn!(irrigation_type = %i, "Unknown irrigation_type in database, ignoring");
            None
        })
    });

    Ok(LawnProfile {
        id: Some(row.get("id")?),
        name: row.get("name")?,
        grass_type,
        usda_zone: row.get("usda_zone")?,
        soil_type,
        lawn_size_sqft: row.get("lawn_size_sqft")?,
        irrigation_type,
        created_at: DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
        updated_at: DateTime::parse_from_rfc3339(&updated_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
    })
}

// Application Queries

impl Database {
    pub fn get_applications_for_profile(&self, profile_id: i64) -> Result<Vec<Application>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT * FROM applications WHERE lawn_profile_id = ?1 ORDER BY application_date DESC",
            )?;
            let apps = stmt
                .query_map([profile_id], row_to_application)?
                .filter_map(|r| r.ok())
                .collect();
            Ok(apps)
        })
    }

    pub fn delete_application(&self, id: i64) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute("DELETE FROM applications WHERE id = ?1", [id])?;
            Ok(())
        })
    }
}

fn row_to_application(row: &Row) -> rusqlite::Result<Application> {
    let app_type_str: String = row.get("application_type")?;
    let date_str: String = row.get("application_date")?;
    let created_at_str: String = row.get("created_at")?;

    let soil_temp: Option<f64> = row.get("soil_temp_10cm_f")?;
    let ambient_temp: Option<f64> = row.get("ambient_temp_f")?;
    let humidity: Option<f64> = row.get("humidity_percent")?;
    let moisture: Option<f64> = row.get("soil_moisture")?;

    let weather = if soil_temp.is_some()
        || ambient_temp.is_some()
        || humidity.is_some()
        || moisture.is_some()
    {
        Some(WeatherSnapshot {
            soil_temp_10cm_f: soil_temp,
            ambient_temp_f: ambient_temp,
            humidity_percent: humidity,
            soil_moisture: moisture,
        })
    } else {
        None
    };

    let application_type = ApplicationType::from_str(&app_type_str).unwrap_or_else(|| {
        warn!(
            application_type = %app_type_str,
            "Unknown application_type in database, defaulting to Other"
        );
        ApplicationType::Other
    });

    Ok(Application {
        id: Some(row.get("id")?),
        lawn_profile_id: row.get("lawn_profile_id")?,
        application_type,
        product_name: row.get("product_name")?,
        application_date: NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
            .unwrap_or_else(|_| chrono::Local::now().date_naive()),
        rate_per_1000sqft: row.get("rate_per_1000sqft")?,
        coverage_sqft: row.get("coverage_sqft")?,
        notes: row.get("notes")?,
        weather_snapshot: weather,
        created_at: DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
    })
}

// Environmental Cache Queries

impl Database {
    pub fn cache_environmental_reading(&self, reading: &EnvironmentalReading) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                r#"
                INSERT OR REPLACE INTO environmental_cache
                    (timestamp, source, soil_temp_5_f, soil_temp_10_f, soil_temp_20_f,
                     soil_temp_50_f, soil_temp_100_f, soil_moisture_5, soil_moisture_10,
                     soil_moisture_20, soil_moisture_50, soil_moisture_100,
                     ambient_temp_f, humidity_percent, precipitation_mm, fetched_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
                "#,
                params![
                    reading.timestamp.to_rfc3339(),
                    format!("{:?}", reading.source),
                    reading.soil_temp_5_f,
                    reading.soil_temp_10_f,
                    reading.soil_temp_20_f,
                    reading.soil_temp_50_f,
                    reading.soil_temp_100_f,
                    reading.soil_moisture_5,
                    reading.soil_moisture_10,
                    reading.soil_moisture_20,
                    reading.soil_moisture_50,
                    reading.soil_moisture_100,
                    reading.ambient_temp_f,
                    reading.humidity_percent,
                    reading.precipitation_mm,
                    Utc::now().to_rfc3339(),
                ],
            )?;
            Ok(())
        })
    }
}

trait OptionalExt<T> {
    fn optional(self) -> rusqlite::Result<Option<T>>;
}

impl<T> OptionalExt<T> for rusqlite::Result<T> {
    fn optional(self) -> rusqlite::Result<Option<T>> {
        match self {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}
