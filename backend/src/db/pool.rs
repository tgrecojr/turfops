use crate::error::Result;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::PgPool;

pub async fn create_pool(options: PgConnectOptions) -> Result<PgPool> {
    let max_conn: u32 = std::env::var("DB_MAX_CONNECTIONS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);

    let pool = PgPoolOptions::new()
        .max_connections(max_conn)
        .connect_with(options)
        .await?;

    // Run migrations
    sqlx::migrate!("./src/db/migrations")
        .run(&pool)
        .await
        .map_err(|e| crate::error::TurfOpsError::Config(format!("Migration failed: {}", e)))?;

    tracing::info!(
        "Database connected and migrations applied (max_connections={})",
        max_conn
    );
    Ok(pool)
}
