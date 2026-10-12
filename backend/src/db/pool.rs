use crate::error::Result;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::PgPool;
use std::future::Future;
use std::time::Duration;

/// Connect (retrying while PostgreSQL comes up, `startup_delays`) and run migrations.
// @spec DEPLOY-START-007, DEPLOY-START-002, DEPLOY-START-003, DEPLOY-CONFIG-008
pub async fn create_pool(options: PgConnectOptions, max_conn: u32) -> Result<PgPool> {
    let pool = with_retry(&startup_delays(), unreachable, || {
        PgPoolOptions::new()
            .max_connections(max_conn)
            .connect_with(options.clone())
    })
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

/// Waits between attempts to reach PostgreSQL at startup.
pub fn startup_delays() -> Vec<Duration> {
    [2, 4, 8, 16, 30].map(Duration::from_secs).to_vec()
}

/// PostgreSQL could not be reached yet (down, still starting, network not up). A wrong
/// password or database name is not worth waiting out.
fn unreachable(err: &sqlx::Error) -> bool {
    match err {
        sqlx::Error::Io(_) | sqlx::Error::PoolTimedOut | sqlx::Error::Tls(_) => true,
        // 57P03 cannot_connect_now: the server is starting up or shutting down.
        sqlx::Error::Database(db) => db.code().as_deref() == Some("57P03"),
        _ => false,
    }
}

/// Run `attempt` until it succeeds, fails in a way `retryable` rejects, or `delays` run
/// out, waiting each delay between tries.
pub async fn with_retry<T, E, F, Fut>(
    delays: &[Duration],
    retryable: impl Fn(&E) -> bool,
    mut attempt: F,
) -> std::result::Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = std::result::Result<T, E>>,
    E: std::fmt::Display,
{
    let mut waits = delays.iter();
    loop {
        match attempt().await {
            Ok(value) => return Ok(value),
            Err(err) => match waits.next().filter(|_| retryable(&err)) {
                Some(wait) => {
                    tracing::warn!("Database not reachable ({err}); retrying in {wait:?}");
                    tokio::time::sleep(*wait).await;
                }
                None => return Err(err),
            },
        }
    }
}

#[cfg(test)]
mod tests;
