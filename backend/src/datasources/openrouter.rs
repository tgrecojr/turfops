use crate::config::OpenRouterConfig;
use crate::error::{Result, TurfOpsError};
use crate::models::plant::{PlantMaintenancePlan, PlantType};
use serde::Serialize;
use serde_json::json;

const APP_URL: &str = "https://github.com/tgrecojr/turfops";
const APP_TITLE: &str = "TurfOps";

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
        Self {
            client: reqwest::Client::new(),
            config,
        }
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
        if !self.config.enabled {
            return Err(TurfOpsError::DataSourceUnavailable(
                "OpenRouter is disabled".into(),
            ));
        }

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

        let body = json!({
            "model": self.config.model,
            "messages": [
                {
                    "role": "system",
                    "content": [
                        {
                            "type": "text",
                            "text": SYSTEM_PROMPT,
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
                    "name": "plant_maintenance_plan",
                    "strict": true,
                    "schema": plan_json_schema(),
                }
            },
            "temperature": 0.2,
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

        serde_json::from_str::<PlantMaintenancePlan>(content).map_err(|e| {
            TurfOpsError::DataSourceUnavailable(format!(
                "OpenRouter plan JSON invalid: {} (raw: {})",
                e, content
            ))
        })
    }
}

const SYSTEM_PROMPT: &str = "You are a landscape-maintenance assistant helping a HOMEOWNER \
(not a professional arborist or horticulturist) care for the plants, shrubs, and bushes around \
their lawn. The user will give you a common name OR a genus/species, their USDA hardiness zone, \
and the plant type they selected.\n\n\
Your job:\n\
1. Identify the plant. If the input is ambiguous, pick the most common residential variety and \
   set identification_confidence to \"Medium\" or \"Low\". Always populate scientific_name when you \
   can.\n\
2. Produce a practical YEAR-ROUND maintenance plan. Tasks should be at a homeowner level: \
   general pruning, fertilizing, mulching, watering guidance, pest inspection, deadheading, \
   winter protection. Do NOT recommend anything requiring a certified applicator license or \
   heavy equipment beyond hand pruners / loppers / a bow rake.\n\
3. Timing: every task must include a window as MM-DD strings. Windows are calendar ranges that \
   repeat every year. Tune windows to the USDA zone given. If the plant needs multiple prunings \
   per year, emit one MaintenanceTask per window.\n\
4. Keep task descriptions concrete and short (1-3 sentences). Include WHY (e.g., \"prune after \
   bloom so you don't cut off next year's flower buds\").\n\
5. Add warnings only if they are homeowner-relevant (pet toxicity, invasive-in-some-states, \
   thorns, allergenic sap).\n\n\
Output MUST conform to the provided JSON schema exactly. Do not include any prose outside the \
JSON. If you cannot identify the plant at all, return a plan with identification_confidence \
\"Low\", a summary explaining why, and an empty tasks array.";

fn plan_json_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": [
            "identified_name",
            "scientific_name",
            "identification_confidence",
            "summary",
            "tasks",
            "warnings"
        ],
        "properties": {
            "identified_name": { "type": "string" },
            "scientific_name": { "type": ["string", "null"] },
            "identification_confidence": {
                "type": "string",
                "enum": ["High", "Medium", "Low"]
            },
            "summary": { "type": "string" },
            "warnings": {
                "type": "array",
                "items": { "type": "string" }
            },
            "tasks": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": [
                        "task_type",
                        "window_start_month_day",
                        "window_end_month_day",
                        "frequency",
                        "description",
                        "severity",
                        "zone_note"
                    ],
                    "properties": {
                        "task_type": {
                            "type": "string",
                            "enum": [
                                "Pruning",
                                "Fertilizing",
                                "Mulching",
                                "Watering",
                                "PestInspection",
                                "Deadheading",
                                "WinterProtection",
                                "Other"
                            ]
                        },
                        "window_start_month_day": {
                            "type": "string",
                            "pattern": "^(0[1-9]|1[0-2])-(0[1-9]|[12][0-9]|3[01])$"
                        },
                        "window_end_month_day": {
                            "type": "string",
                            "pattern": "^(0[1-9]|1[0-2])-(0[1-9]|[12][0-9]|3[01])$"
                        },
                        "frequency": {
                            "type": "string",
                            "enum": ["Once", "Twice", "Monthly", "AsNeeded"]
                        },
                        "description": { "type": "string" },
                        "severity": {
                            "type": "string",
                            "enum": ["Info", "Advisory", "Warning", "Critical"]
                        },
                        "zone_note": { "type": ["string", "null"] }
                    }
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_config() -> OpenRouterConfig {
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
