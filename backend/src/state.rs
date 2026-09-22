use crate::datasources::weather::ClimateRecord;
use crate::datasources::OpenRouterClient;
use crate::logic::data_sync::DataSyncService;
use crate::logic::rules::RulesEngine;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

/// Multi-year daily climate record from the lake with the time it was read. Unlike the
/// other lake reads this one spans 15 years of hourly data, so it is memoized briefly.
pub type ClimateCache = Arc<RwLock<Option<(Instant, Arc<ClimateRecord>)>>>;

#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::PgPool,
    pub rules_engine: Arc<RulesEngine>,
    pub sync_service: Arc<RwLock<DataSyncService>>,
    pub openrouter: Option<Arc<OpenRouterClient>>,
    pub climate_cache: ClimateCache,
    /// Whether `/mcp` is mounted (reported by the health check).
    pub mcp_enabled: bool,
}

impl AppState {
    pub fn new(
        pool: sqlx::PgPool,
        sync_service: DataSyncService,
        openrouter: Option<OpenRouterClient>,
        mcp_enabled: bool,
    ) -> Self {
        Self {
            pool,
            rules_engine: Arc::new(RulesEngine::new()),
            sync_service: Arc::new(RwLock::new(sync_service)),
            openrouter: openrouter.map(Arc::new),
            climate_cache: Arc::new(RwLock::new(None)),
            mcp_enabled,
        }
    }
}
