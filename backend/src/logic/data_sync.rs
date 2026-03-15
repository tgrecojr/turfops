use crate::config::Config;
use crate::datasources::{HomeAssistantClient, OpenWeatherMapClient, SoilDataClient};
use crate::db::queries;
use crate::models::{DataSource, EnvironmentalReading, EnvironmentalSummary, WeatherForecast};
use chrono::Utc;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::Instant;

/// How long before sensor data (SoilData + Home Assistant) is considered stale.
const SENSOR_STALENESS_SECS: u64 = 5 * 60; // 5 minutes

/// How long before forecast data (OpenWeatherMap) is considered stale.
const FORECAST_STALENESS_SECS: u64 = 30 * 60; // 30 minutes

pub struct DataSyncService {
    pool: PgPool,
    soildata_client: Option<SoilDataClient>,
    homeassistant_client: Option<HomeAssistantClient>,
    openweathermap_client: Option<OpenWeatherMapClient>,
    current_summary: Arc<RwLock<EnvironmentalSummary>>,
    current_forecast: Arc<RwLock<Option<WeatherForecast>>>,
    last_sensor_refresh: Option<Instant>,
    last_forecast_refresh: Option<Instant>,
}

impl DataSyncService {
    /// Create a new DataSyncService and connect to external datasources.
    /// Call this once at startup; the service is then stored in AppState.
    pub async fn initialize(config: &Config, pool: PgPool) -> Self {
        let homeassistant_client = if !config.homeassistant.token.is_empty() {
            let client = HomeAssistantClient::new(config.homeassistant.clone());
            match client.test_connection().await {
                Ok(true) => {
                    tracing::info!(
                        url = %config.homeassistant.url,
                        "Connected to Home Assistant"
                    );
                }
                Ok(false) => {
                    tracing::warn!(
                        url = %config.homeassistant.url,
                        "Home Assistant connection test failed - check HA_URL and HA_TOKEN"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        url = %config.homeassistant.url,
                        error = %e,
                        "Cannot reach Home Assistant - check HA_URL is correct (expected base URL, e.g. http://homeassistant.local:8123)"
                    );
                }
            }
            Some(client)
        } else {
            tracing::warn!(
                "Home Assistant token not configured - ambient data will be unavailable"
            );
            None
        };

        let openweathermap_client = config
            .openweathermap
            .as_ref()
            .filter(|c| c.enabled && !c.api_key.is_empty())
            .map(|c| {
                tracing::info!("OpenWeatherMap client configured for forecast data");
                OpenWeatherMapClient::new(c.clone())
            });

        if openweathermap_client.is_none() {
            tracing::info!(
                "OpenWeatherMap not configured - forecast-based recommendations will be limited"
            );
        }

        let soildata_client =
            match SoilDataClient::connect(&config.soildata, config.noaa.station_wbanno).await {
                Ok(client) => {
                    tracing::info!("Connected to SoilData PostgreSQL");
                    Some(client)
                }
                Err(e) => {
                    tracing::warn!("Failed to connect to SoilData: {}", e);
                    None
                }
            };

        Self {
            pool,
            soildata_client,
            homeassistant_client,
            openweathermap_client,
            current_summary: Arc::new(RwLock::new(EnvironmentalSummary::default())),
            current_forecast: Arc::new(RwLock::new(None)),
            last_sensor_refresh: None,
            last_forecast_refresh: None,
        }
    }

    /// Return cached summary if fresh, otherwise fetch from datasources first.
    /// Sensor data refreshes after 5 minutes, forecast after 30 minutes.
    pub async fn get_or_refresh(&mut self) -> crate::error::Result<EnvironmentalSummary> {
        let sensor_stale = self.is_sensor_stale();
        let forecast_stale = self.is_forecast_stale();

        if sensor_stale || forecast_stale {
            self.refresh_internal(sensor_stale, forecast_stale).await?;
        }

        let summary = self.current_summary.read().await.clone();
        Ok(summary)
    }

    /// Always fetch fresh data from all datasources, ignoring cache age.
    /// Used by the explicit refresh button in the frontend.
    pub async fn force_refresh(&mut self) -> crate::error::Result<EnvironmentalSummary> {
        self.refresh_internal(true, true).await
    }

    /// Return the current cached summary without triggering any fetch.
    pub async fn cached_summary(&self) -> EnvironmentalSummary {
        self.current_summary.read().await.clone()
    }

    pub async fn check_connections(&self) -> ConnectionStatus {
        let mut status = ConnectionStatus::default();

        if let Some(ref client) = self.soildata_client {
            status.soildata = client.test_connection().await.unwrap_or(false);
        }

        if let Some(ref client) = self.homeassistant_client {
            status.homeassistant = client.test_connection().await.unwrap_or(false);
        }

        if let Some(ref client) = self.openweathermap_client {
            status.openweathermap = client.test_connection().await.unwrap_or(false);
        }

        status
    }

    fn is_sensor_stale(&self) -> bool {
        match self.last_sensor_refresh {
            None => true,
            Some(t) => t.elapsed().as_secs() >= SENSOR_STALENESS_SECS,
        }
    }

    fn is_forecast_stale(&self) -> bool {
        match self.last_forecast_refresh {
            None => true,
            Some(t) => t.elapsed().as_secs() >= FORECAST_STALENESS_SECS,
        }
    }

    async fn refresh_internal(
        &mut self,
        refresh_sensors: bool,
        refresh_forecast: bool,
    ) -> crate::error::Result<EnvironmentalSummary> {
        let mut summary = EnvironmentalSummary::default();
        let mut combined_reading = EnvironmentalReading::new(DataSource::Cached);

        if refresh_sensors {
            // Fetch soil data from PostgreSQL
            if let Some(ref client) = self.soildata_client {
                match client.fetch_summary().await {
                    Ok(soil_summary) => {
                        summary = soil_summary;
                        if let Some(ref current) = summary.current {
                            combined_reading.soil_temp_5_f = current.soil_temp_5_f;
                            combined_reading.soil_temp_10_f = current.soil_temp_10_f;
                            combined_reading.soil_temp_20_f = current.soil_temp_20_f;
                            combined_reading.soil_temp_50_f = current.soil_temp_50_f;
                            combined_reading.soil_temp_100_f = current.soil_temp_100_f;
                            combined_reading.soil_moisture_5 = current.soil_moisture_5;
                            combined_reading.soil_moisture_10 = current.soil_moisture_10;
                            combined_reading.soil_moisture_20 = current.soil_moisture_20;
                            combined_reading.soil_moisture_50 = current.soil_moisture_50;
                            combined_reading.soil_moisture_100 = current.soil_moisture_100;
                            combined_reading.precipitation_mm = current.precipitation_mm;
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to fetch soil data: {}", e);
                    }
                }
            }

            // Fetch ambient data from Home Assistant
            if let Some(ref client) = self.homeassistant_client {
                match client.fetch_current().await {
                    Ok(ha_reading) => {
                        if ha_reading.ambient_temp_f.is_some() {
                            combined_reading.ambient_temp_f = ha_reading.ambient_temp_f;
                        }
                        if ha_reading.humidity_percent.is_some() {
                            combined_reading.humidity_percent = ha_reading.humidity_percent;
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to fetch Home Assistant data: {}", e);
                    }
                }
            }

            combined_reading.timestamp = Utc::now();
            summary.current = Some(combined_reading.clone());
            summary.last_updated = Some(Utc::now());
            self.last_sensor_refresh = Some(Instant::now());

            // Cache the reading to app DB
            if let Err(e) =
                queries::cache_environmental_reading(&self.pool, &combined_reading).await
            {
                tracing::error!("Failed to cache environmental reading: {}", e);
            }
        } else {
            // Keep existing sensor data
            summary = self.current_summary.read().await.clone();
        }

        if refresh_forecast {
            if let Some(ref client) = self.openweathermap_client {
                match client.fetch_forecast().await {
                    Ok(forecast) => {
                        summary.forecast = Some(forecast.clone());
                        let mut current_forecast = self.current_forecast.write().await;
                        *current_forecast = Some(forecast);
                        self.last_forecast_refresh = Some(Instant::now());
                        tracing::debug!("Weather forecast updated");
                    }
                    Err(e) => {
                        tracing::warn!("Failed to fetch weather forecast: {}", e);
                    }
                }
            }
        } else {
            // Keep existing forecast
            let existing_forecast = self.current_forecast.read().await.clone();
            if summary.forecast.is_none() {
                summary.forecast = existing_forecast;
            }
        }

        // Update shared cached summary
        let mut current = self.current_summary.write().await;
        *current = summary.clone();

        Ok(summary)
    }
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct ConnectionStatus {
    pub soildata: bool,
    pub homeassistant: bool,
    pub openweathermap: bool,
}
