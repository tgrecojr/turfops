use crate::config::OpenRouterConfig;
use crate::error::{Result, TurfOpsError};
use crate::models::plant::{PlantMaintenancePlan, PlantType};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::json;
use std::time::Duration;

const APP_URL: &str = "https://github.com/tgrecojr/turfops";
const APP_TITLE: &str = "TurfOps";
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

mod plant;
mod product;
use plant::{plan_json_schema, SYSTEM_PROMPT};
pub use product::ProductProfileRequest;

pub struct OpenRouterClient {
    client: reqwest::Client,
    config: OpenRouterConfig,
}

#[derive(Debug, Serialize)]
pub struct PlantPlanRequest<'a> {
    pub input: &'a str,
    pub usda_zone: &'a str,
    pub plant_type: PlantType,
    pub location: Option<&'a str>,
}

impl OpenRouterClient {
    pub fn new(config: OpenRouterConfig) -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .build()
            .expect("failed to build OpenRouter HTTP client");
        Self { client, config }
    }

    pub fn model(&self) -> &str {
        &self.config.model
    }

    /// Generate a structured plant maintenance plan via an LLM.
    /// The response is validated against a JSON schema by OpenRouter.
    pub async fn generate_plant_plan(
        &self,
        req: PlantPlanRequest<'_>,
    ) -> Result<PlantMaintenancePlan> {
        let user_prompt = format!(
            "Identify the plant and produce a year-round maintenance plan.\n\
             Input: {}\n\
             USDA hardiness zone: {}\n\
             Plant type (user-reported): {}\n\
             Location on property: {}\n",
            req.input,
            req.usda_zone,
            req.plant_type,
            req.location.unwrap_or("unspecified"),
        );

        self.complete_json(
            SYSTEM_PROMPT,
            user_prompt,
            "plant_maintenance_plan",
            plan_json_schema(),
            0.2,
        )
        .await
    }

    /// One strict-JSON-schema chat completion: the system prompt is cached, the reply is
    /// validated by OpenRouter against `schema`, then deserialized into `T`.
    pub(super) async fn complete_json<T: DeserializeOwned>(
        &self,
        system_prompt: &str,
        user_prompt: String,
        schema_name: &str,
        schema: serde_json::Value,
        temperature: f64,
    ) -> Result<T> {
        if !self.config.enabled {
            return Err(TurfOpsError::DataSourceUnavailable(
                "OpenRouter is disabled".into(),
            ));
        }

        let body = json!({
            "model": self.config.model,
            "messages": [
                {
                    "role": "system",
                    "content": [
                        {
                            "type": "text",
                            "text": system_prompt,
                            "cache_control": { "type": "ephemeral" }
                        }
                    ]
                },
                {
                    "role": "user",
                    "content": user_prompt,
                }
            ],
            "response_format": {
                "type": "json_schema",
                "json_schema": {
                    "name": schema_name,
                    "strict": true,
                    "schema": schema,
                }
            },
            "temperature": temperature,
        });

        let url = format!("{}/chat/completions", self.config.base_url);
        let response = self
            .client
            .post(&url)
            .bearer_auth(&self.config.api_key)
            .header("HTTP-Referer", APP_URL)
            .header("X-Title", APP_TITLE)
            .json(&body)
            .send()
            .await
            .map_err(|e| TurfOpsError::DataSourceUnavailable(format!("OpenRouter: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(TurfOpsError::DataSourceUnavailable(format!(
                "OpenRouter returned {}: {}",
                status, body
            )));
        }

        let raw: serde_json::Value = response.json().await.map_err(|e| {
            TurfOpsError::DataSourceUnavailable(format!("OpenRouter response parse: {}", e))
        })?;

        let content = raw
            .pointer("/choices/0/message/content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                TurfOpsError::DataSourceUnavailable(format!(
                    "OpenRouter response missing content: {}",
                    raw
                ))
            })?;

        serde_json::from_str::<T>(content).map_err(|e| {
            TurfOpsError::DataSourceUnavailable(format!(
                "OpenRouter {} JSON invalid: {} (raw: {})",
                schema_name, e, content
            ))
        })
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    pub(super) fn sample_config() -> OpenRouterConfig {
        OpenRouterConfig {
            api_key: "test_key".to_string(),
            model: "anthropic/claude-haiku-4-5".to_string(),
            enabled: true,
            base_url: "http://localhost:9999/v1".to_string(),
        }
    }

    #[test]
    fn client_creation() {
        let client = OpenRouterClient::new(sample_config());
        assert_eq!(client.model(), "anthropic/claude-haiku-4-5");
    }

    #[test]
    fn disabled_client_errors() {
        let mut cfg = sample_config();
        cfg.enabled = false;
        let client = OpenRouterClient::new(cfg);
        let req = PlantPlanRequest {
            input: "hydrangea",
            usda_zone: "7a",
            plant_type: PlantType::Shrub,
            location: None,
        };
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let result = rt.block_on(client.generate_plant_plan(req));
        assert!(matches!(
            result,
            Err(TurfOpsError::DataSourceUnavailable(_))
        ));
    }

    #[test]
    fn schema_has_required_fields() {
        let schema = plan_json_schema();
        let required = schema["required"].as_array().unwrap();
        let names: Vec<&str> = required.iter().filter_map(|v| v.as_str()).collect();
        assert!(names.contains(&"tasks"));
        assert!(names.contains(&"identification_confidence"));
    }
}
