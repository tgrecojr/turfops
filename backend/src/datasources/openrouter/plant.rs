//! Prompt and strict JSON schema for landscape-plant maintenance plans.
use serde_json::json;

pub(super) const SYSTEM_PROMPT: &str = "You are a landscape-maintenance assistant helping a HOMEOWNER \
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

pub(super) fn plan_json_schema() -> serde_json::Value {
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
