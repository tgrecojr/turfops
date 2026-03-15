use crate::api::recommendations::RecommendationState;
use crate::config::Config;
use crate::logic::data_sync::DataSyncService;
use crate::logic::rules::RulesEngine;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::PgPool,
    pub config: Config,
    pub rules_engine: Arc<RulesEngine>,
    pub sync_service: Arc<Mutex<DataSyncService>>,
    pub recommendation_states: Arc<RwLock<HashMap<String, RecommendationState>>>,
}

impl AppState {
    pub fn new(pool: sqlx::PgPool, config: Config, sync_service: DataSyncService) -> Self {
        Self {
            pool,
            config,
            rules_engine: Arc::new(RulesEngine::new()),
            sync_service: Arc::new(Mutex::new(sync_service)),
            recommendation_states: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}
