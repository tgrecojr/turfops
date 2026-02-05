use crate::config::Config;
use crate::datasources::{HomeAssistantClient, OpenWeatherMapClient, SoilDataClient};
use crate::db::Database;
use crate::error::Result;
use crate::models::{DataSource, EnvironmentalReading, EnvironmentalSummary, WeatherForecast};
use chrono::Utc;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct DataSyncService {
    config: Config,
    db: Database,
    soildata_client: Option<SoilDataClient>,
    homeassistant_client: Option<HomeAssistantClient>,
    openweathermap_client: Option<OpenWeatherMapClient>,
    current_summary: Arc<RwLock<EnvironmentalSummary>>,
    current_forecast: Arc<RwLock<Option<WeatherForecast>>>,
}

impl DataSyncService {
    pub fn new(config: Config, db: Database) -> Self {
        // Only create HA client if token is configured
        let homeassistant_client = if !config.homeassistant.token.is_empty() {
            Some(HomeAssistantClient::new(config.homeassistant.clone()))
        } else {
            tracing::warn!(
                "Home Assistant token not configured - ambient data will be unavailable"
            );
            None
        };

        // Create OpenWeatherMap client if configured and enabled
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

        Self {
            config,
            db,
            soildata_client: None,
            homeassistant_client,
            openweathermap_client,
            current_summary: Arc::new(RwLock::new(EnvironmentalSummary::default())),
            current_forecast: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn initialize(&mut self) -> Result<()> {
        // Connect to SoilData PostgreSQL
        match SoilDataClient::connect(&self.config.soildata, self.config.noaa.station_wbanno).await
        {
            Ok(client) => {
                self.soildata_client = Some(client);
                tracing::info!("Connected to SoilData PostgreSQL");
            }
            Err(e) => {
                tracing::warn!("Failed to connect to SoilData: {}", e);
            }
        }

        // Initial data fetch
        self.refresh().await?;

        Ok(())
    }

    pub async fn refresh(&mut self) -> Result<EnvironmentalSummary> {
        let mut summary = EnvironmentalSummary::default();
        let mut combined_reading = EnvironmentalReading::new(DataSource::Cached);

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

        // Fetch ambient data from Home Assistant (overrides NOAA ambient if available)
        if let Some(ref client) = self.homeassistant_client {
            match client.fetch_current().await {
                Ok(ha_reading) => {
                    // Prefer local sensor for ambient conditions
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

        // Fetch weather forecast
        if let Some(ref client) = self.openweathermap_client {
            match client.fetch_forecast().await {
                Ok(forecast) => {
                    summary.forecast = Some(forecast.clone());
                    let mut current_forecast = self.current_forecast.write().await;
                    *current_forecast = Some(forecast);
                    tracing::debug!("Weather forecast updated");
                }
                Err(e) => {
                    tracing::warn!("Failed to fetch weather forecast: {}", e);
                }
            }
        }

        // Cache the reading
        self.db.cache_environmental_reading(&combined_reading)?;

        // Update shared state
        let mut current = self.current_summary.write().await;
        *current = summary.clone();

        Ok(summary)
    }

    /// Refresh only the weather forecast
    pub async fn refresh_forecast(&self) -> Result<Option<WeatherForecast>> {
        if let Some(ref client) = self.openweathermap_client {
            let forecast = client.fetch_forecast().await?;
            let mut current_forecast = self.current_forecast.write().await;
            *current_forecast = Some(forecast.clone());
            Ok(Some(forecast))
        } else {
            Ok(None)
        }
    }

    pub async fn get_current_forecast(&self) -> Option<WeatherForecast> {
        self.current_forecast.read().await.clone()
    }

    pub async fn get_current_summary(&self) -> EnvironmentalSummary {
        self.current_summary.read().await.clone()
    }

    pub fn get_cached_readings(&self, hours: u32) -> Result<Vec<EnvironmentalReading>> {
        self.db.get_cached_readings(hours)
    }

    pub async fn check_connections(&self) -> ConnectionStatus {
        let mut status = ConnectionStatus::default();

        // Check SoilData
        if let Some(ref client) = self.soildata_client {
            status.soildata = client.test_connection().await.unwrap_or(false);
        }

        // Check Home Assistant
        if let Some(ref client) = self.homeassistant_client {
            status.homeassistant = client.test_connection().await.unwrap_or(false);
        }

        // Check OpenWeatherMap
        if let Some(ref client) = self.openweathermap_client {
            status.openweathermap = client.test_connection().await.unwrap_or(false);
        }

        status
    }
}

#[derive(Debug, Clone, Default)]
pub struct ConnectionStatus {
    pub soildata: bool,
    pub homeassistant: bool,
    pub openweathermap: bool,
}

impl ConnectionStatus {
    pub fn all_connected(&self) -> bool {
        self.soildata && self.homeassistant && self.openweathermap
    }

    pub fn any_connected(&self) -> bool {
        self.soildata || self.homeassistant || self.openweathermap
    }

    pub fn core_connected(&self) -> bool {
        // Core data sources (soil and ambient) are connected
        self.soildata && self.homeassistant
    }
}
