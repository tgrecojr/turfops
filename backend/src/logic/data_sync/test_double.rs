//! A `DataSyncService` for tests that need an `AppState`: no Home Assistant, no forecast,
//! and the weather lake only when the test supplies a client.

use super::probes::ProbeCache;
use super::sync_state::SyncState;
use super::DataSyncService;
use crate::clock::Clock;
use crate::datasources::WeatherLakeClient;
use sqlx::PgPool;
use std::sync::Mutex;

impl DataSyncService {
    /// A service with only the lake client given (or none, as when the lake is not
    /// configured) and no external connections.
    pub(crate) fn for_tests(
        pool: PgPool,
        weather_client: Option<WeatherLakeClient>,
        clock: Clock,
    ) -> Self {
        Self {
            pool,
            weather_client,
            homeassistant_client: None,
            openweathermap_client: None,
            state: SyncState::default(),
            probes: Mutex::new(ProbeCache::default()),
            clock,
        }
    }
}
