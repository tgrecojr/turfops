use crate::db::Database;
use crate::error::Result;

const MIGRATIONS: &[&str] = &[
    // Migration 1: Initial schema
    r#"
    CREATE TABLE IF NOT EXISTS lawn_profiles (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT NOT NULL,
        grass_type TEXT NOT NULL,
        usda_zone TEXT NOT NULL,
        soil_type TEXT,
        lawn_size_sqft REAL,
        irrigation_type TEXT,
        created_at TEXT NOT NULL DEFAULT (datetime('now')),
        updated_at TEXT NOT NULL DEFAULT (datetime('now'))
    );

    CREATE TABLE IF NOT EXISTS applications (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        lawn_profile_id INTEGER NOT NULL REFERENCES lawn_profiles(id) ON DELETE CASCADE,
        application_type TEXT NOT NULL,
        product_name TEXT,
        application_date TEXT NOT NULL,
        rate_per_1000sqft REAL,
        coverage_sqft REAL,
        notes TEXT,
        soil_temp_10cm_f REAL,
        ambient_temp_f REAL,
        humidity_percent REAL,
        soil_moisture REAL,
        created_at TEXT NOT NULL DEFAULT (datetime('now')),
        UNIQUE(lawn_profile_id, application_type, application_date)
    );

    CREATE TABLE IF NOT EXISTS environmental_cache (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        timestamp TEXT NOT NULL UNIQUE,
        source TEXT NOT NULL,
        soil_temp_5_f REAL,
        soil_temp_10_f REAL,
        soil_temp_20_f REAL,
        soil_temp_50_f REAL,
        soil_temp_100_f REAL,
        soil_moisture_5 REAL,
        soil_moisture_10 REAL,
        soil_moisture_20 REAL,
        soil_moisture_50 REAL,
        soil_moisture_100 REAL,
        ambient_temp_f REAL,
        humidity_percent REAL,
        precipitation_mm REAL,
        fetched_at TEXT NOT NULL DEFAULT (datetime('now'))
    );

    CREATE TABLE IF NOT EXISTS settings (
        key TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS schema_migrations (
        version INTEGER PRIMARY KEY,
        applied_at TEXT NOT NULL DEFAULT (datetime('now'))
    );
    "#,
    // Migration 2: Add indexes
    r#"
    CREATE INDEX IF NOT EXISTS idx_applications_lawn_profile_id
        ON applications(lawn_profile_id);
    CREATE INDEX IF NOT EXISTS idx_applications_date
        ON applications(application_date);
    CREATE INDEX IF NOT EXISTS idx_environmental_cache_timestamp
        ON environmental_cache(timestamp);
    "#,
];

pub fn run(db: &Database) -> Result<()> {
    db.with_conn_mut(|conn| {
        // Ensure schema_migrations table exists
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            "#,
        )?;

        // Get current version
        let current_version: i32 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        // Apply pending migrations
        for (i, migration) in MIGRATIONS.iter().enumerate() {
            let version = (i + 1) as i32;
            if version > current_version {
                tracing::info!("Applying migration {}", version);
                conn.execute_batch(migration)?;
                conn.execute(
                    "INSERT INTO schema_migrations (version) VALUES (?1)",
                    [version],
                )?;
            }
        }

        Ok(())
    })
}
