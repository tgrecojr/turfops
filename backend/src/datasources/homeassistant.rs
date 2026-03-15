use crate::config::{HomeAssistantConfig, TemperatureUnit};
use crate::error::{Result, TurfOpsError};
use crate::models::{celsius_to_fahrenheit, DataSource, EnvironmentalReading};
use chrono::Utc;
use serde::Deserialize;

pub struct HomeAssistantClient {
    client: reqwest::Client,
    config: HomeAssistantConfig,
}

#[derive(Debug, Deserialize)]
struct EntityState {
    state: String,
    #[allow(dead_code)]
    entity_id: String,
}

impl HomeAssistantClient {
    pub fn new(config: HomeAssistantConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            config,
        }
    }

    pub async fn fetch_current(&self) -> Result<EnvironmentalReading> {
        let mut reading = EnvironmentalReading::new(DataSource::HomeAssistant);

        // Fetch temperature
        match self.get_entity_state(&self.config.temperature_entity).await {
            Ok(Some(temp)) => {
                let temp_f = match self.config.temperature_unit {
                    TemperatureUnit::Celsius => celsius_to_fahrenheit(temp),
                    TemperatureUnit::Fahrenheit => temp,
                };
                reading.ambient_temp_f = Some(temp_f);
            }
            Ok(None) => {
                tracing::warn!(
                    entity = %self.config.temperature_entity,
                    "Home Assistant temperature entity returned non-numeric state"
                );
            }
            Err(e) => {
                tracing::warn!(
                    entity = %self.config.temperature_entity,
                    error = %e,
                    "Failed to fetch temperature from Home Assistant"
                );
            }
        }

        // Fetch humidity
        match self.get_entity_state(&self.config.humidity_entity).await {
            Ok(Some(humidity)) => {
                reading.humidity_percent = Some(humidity);
            }
            Ok(None) => {
                tracing::warn!(
                    entity = %self.config.humidity_entity,
                    "Home Assistant humidity entity returned non-numeric state"
                );
            }
            Err(e) => {
                tracing::warn!(
                    entity = %self.config.humidity_entity,
                    error = %e,
                    "Failed to fetch humidity from Home Assistant"
                );
            }
        }

        reading.timestamp = Utc::now();
        Ok(reading)
    }

    async fn get_entity_state(&self, entity_id: &str) -> Result<Option<f64>> {
        let url = format!("{}/api/states/{}", self.config.url, entity_id);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.config.token))
            .header("Content-Type", "application/json")
            .send()
            .await
            .map_err(|e| {
                TurfOpsError::DataSourceUnavailable(format!(
                    "Home Assistant request to {} failed: {}",
                    url, e
                ))
            })?;

        if !response.status().is_success() {
            return Err(TurfOpsError::DataSourceUnavailable(format!(
                "Home Assistant GET {} returned {}",
                url,
                response.status()
            )));
        }

        let entity: EntityState = response.json().await.map_err(|e| {
            TurfOpsError::DataSourceUnavailable(format!(
                "Failed to parse Home Assistant response: {}",
                e
            ))
        })?;

        Ok(entity.state.parse::<f64>().ok())
    }

    pub async fn test_connection(&self) -> Result<bool> {
        let url = format!("{}/api/", self.config.url);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.config.token))
            .send()
            .await
            .map_err(|e| TurfOpsError::DataSourceUnavailable(format!("Home Assistant: {}", e)))?;

        Ok(response.status().is_success())
    }
}
