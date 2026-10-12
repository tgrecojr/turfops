#[cfg(all(test, feature = "db-tests"))]
mod db_tests;
#[cfg(test)]
mod memo_slot_tests;
mod probes;
#[cfg(test)]
mod service_tests;
#[cfg(test)]
mod source_tests;
mod sync_state;
#[cfg(test)]
mod test_double;
#[cfg(test)]
pub(crate) mod test_env;
#[cfg(test)]
pub(crate) mod test_support;
#[cfg(test)]
mod tests;

use self::probes::{Probe, ProbeCache};
use self::sync_state::{SensorReads, SyncState};
use crate::clock::Clock;
use crate::config::Config;
use crate::datasources::{HomeAssistantClient, OpenWeatherMapClient, WeatherLakeClient};
use crate::models::EnvironmentalSummary;
use chrono::Utc;
use sqlx::PgPool;
use std::sync::Mutex;

pub struct DataSyncService {
    #[allow(dead_code)]
    pool: PgPool,
    weather_client: Option<WeatherLakeClient>,
    homeassistant_client: Option<HomeAssistantClient>,
    openweathermap_client: Option<OpenWeatherMapClient>,
    state: SyncState,
    probes: Mutex<ProbeCache>,
    clock: Clock,
}

impl DataSyncService {
    // @spec SYNC-REFRESH-001, SYNC-REFRESH-007
    /// Create a new DataSyncService with lazy external connections.
    /// Clients are configured but not tested — connections are validated
    /// on first data fetch, avoiding slow startup when external services are down.
    // @spec WX-SRC-CFG-002
    pub async fn initialize(config: &Config, pool: PgPool) -> Self {
        let homeassistant_client = if !config.homeassistant.token.is_empty() {
            tracing::info!(
                url = %config.homeassistant.url,
                "Home Assistant client configured (connection tested on first fetch)"
            );
            Some(HomeAssistantClient::new(config.homeassistant.clone()))
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
                OpenWeatherMapClient::new(c.clone(), config.clock)
            });

        if openweathermap_client.is_none() {
            tracing::info!(
                "OpenWeatherMap not configured - forecast-based recommendations will be limited"
            );
        }

        let weather_client = Some(WeatherLakeClient::new(
            &config.datalake,
            config.noaa.station_wbanno,
        ));
        tracing::info!("Weather data lake client configured (parquet read on first fetch)");

        Self {
            pool,
            weather_client,
            homeassistant_client,
            openweathermap_client,
            state: SyncState::default(),
            probes: Mutex::new(ProbeCache::default()),
            clock: config.clock,
        }
    }

    /// Return the cached summary, refreshing first whichever part is stale.
    /// Sensor data refreshes after 5 minutes, the forecast after 30 (a failed fetch is
    /// retried after 5).
    // @spec SYNC-REFRESH-003, SYNC-REFRESH-004, SYNC-REFRESH-006
    pub async fn get_or_refresh(&mut self) -> crate::error::Result<EnvironmentalSummary> {
        let due = self.state.due(Utc::now());
        self.refresh(due.sensors, due.forecast).await;
        Ok(self.state.summary(Utc::now()))
    }

    /// Always fetch fresh data from all datasources, ignoring cache age.
    /// Used by the explicit refresh button in the frontend.
    pub async fn force_refresh(&mut self) -> crate::error::Result<EnvironmentalSummary> {
        self.refresh(true, true).await;
        Ok(self.state.summary(Utc::now()))
    }

    /// Connection status, probing only the configured sources whose last result is over
    /// 5 minutes old; the summary and its timers are untouched.
    // @spec SYNC-CONN-001, SYNC-CONN-002, SYNC-CONN-006
    pub async fn check_connections(&self) -> ConnectionStatus {
        let recent = |probe| self.probes.lock().ok()?.recent(probe, Utc::now());
        let mut status = ConnectionStatus::default();

        if let Some(ref client) = self.weather_client {
            status.soildata = match recent(Probe::Lake) {
                Some(up) => up,
                None => self.record(Probe::Lake, client.test_connection().await.unwrap_or(false)),
            };
        }
        if let Some(ref client) = self.homeassistant_client {
            status.homeassistant = match recent(Probe::HomeAssistant) {
                Some(up) => up,
                None => self.record(
                    Probe::HomeAssistant,
                    client.test_connection().await.unwrap_or(false),
                ),
            };
        }
        if let Some(ref client) = self.openweathermap_client {
            status.openweathermap = match recent(Probe::Forecast) {
                Some(up) => up,
                None => self.record(
                    Probe::Forecast,
                    client.test_connection().await.unwrap_or(false),
                ),
            };
        }
        status
    }

    fn record(&self, probe: Probe, up: bool) -> bool {
        if let Ok(mut probes) = self.probes.lock() {
            probes.record(probe, up, Utc::now());
        }
        up
    }

    /// Test-only: fetch the forecast with `client` (e.g. one pointed at a local stub).
    #[cfg(all(test, feature = "db-tests"))]
    pub(crate) fn set_forecast_client(&mut self, client: OpenWeatherMapClient) {
        self.openweathermap_client = Some(client);
    }

    /// The same service reading its forecast from `client`.
    #[cfg(all(test, feature = "db-tests"))]
    pub(crate) fn with_forecast_client(mut self, client: OpenWeatherMapClient) -> Self {
        self.set_forecast_client(client);
        self
    }

    /// Provide access to the weather lake client for direct queries (GDD, historical).
    pub fn weather_client(&self) -> Option<&WeatherLakeClient> {
        self.weather_client.as_ref()
    }

    /// The same service with no weather lake, as if none were configured.
    #[cfg(all(test, feature = "db-tests"))]
    pub(crate) fn without_weather_lake(mut self) -> Self {
        self.weather_client = None;
        self
    }

    /// Read the stale parts' sources, then store everything at once: until every read is
    /// in hand nothing changes, so a request dropped mid-read leaves the part stale.
    // @spec SYNC-REFRESH-008, SYNC-FAIL-001
    async fn refresh(&mut self, sensors: bool, forecast: bool) {
        let sensor_reads = if sensors {
            Some(self.read_sensors().await)
        } else {
            None
        };
        let forecast_read = if forecast {
            Some(match self.openweathermap_client {
                Some(ref client) => Some(client.fetch_forecast().await),
                None => None,
            })
        } else {
            None
        };
        let now = Utc::now();
        if let Some(reads) = sensor_reads {
            self.state.store_sensors(now, reads);
        }
        if let Some(fetched) = forecast_read {
            self.state.store_forecast(now, fetched);
        }
    }

    async fn read_sensors(&self) -> SensorReads {
        let lake = match self.weather_client {
            Some(ref client) => Some(client.fetch_summary().await),
            None => None,
        };
        let home_assistant = match self.homeassistant_client {
            Some(ref client) => Some(client.fetch_current().await),
            None => None,
        };
        let gdd = match self.weather_client {
            Some(ref client) => Some(client.fetch_gdd_ytd(self.clock.today()).await),
            None => None,
        };
        SensorReads {
            lake,
            home_assistant,
            gdd,
        }
    }
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct ConnectionStatus {
    pub soildata: bool,
    pub homeassistant: bool,
    pub openweathermap: bool,
}
