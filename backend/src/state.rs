use crate::logic::data_sync::DataSyncService;
use crate::logic::rules::RulesEngine;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::PgPool,
    pub rules_engine: Arc<RulesEngine>,
    pub sync_service: Arc<RwLock<DataSyncService>>,
}

impl AppState {
    pub fn new(pool: sqlx::PgPool, sync_service: DataSyncService) -> Self {
        Self {
            pool,
            rules_engine: Arc::new(RulesEngine::new()),
            sync_service: Arc::new(RwLock::new(sync_service)),
        }
    }
}
