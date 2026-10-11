//! Prompt and strict JSON schema for landscape-plant maintenance plans.
use serde_json::json;

// @spec PLANT-PROMPT-006, PLANT-PROMPT-007
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
   thorns, allergenic sap).\n\
6. Amounts: a description may give agronomic amounts such as a mulch depth or an amount of a \
   nutrient, but never a product's label rate and never any rate or amount of a pesticide \
   (fungicide, herbicide, insecticide, miticide): the product label governs how much to use.\n\
7. A nutrient amount may be given as an amount of a fertilizer or soil-amendment material \
   (for example a cup of 10-10-10 per shrub), never as an amount of a material that is also a \
   pesticide active: for copper sulfate, give only the amount of copper the plant needs.\n\n\
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

#[cfg(test)]
mod prompt_tests {
    use super::SYSTEM_PROMPT;

    // @spec PLANT-PROMPT-006
    #[test]
    fn the_prompt_allows_agronomic_amounts_but_no_label_or_pesticide_rates() {
        let p = SYSTEM_PROMPT.to_lowercase();
        assert!(p.contains("mulch depth"), "agronomic amounts are allowed");
        assert!(p.contains("label rate"), "never a product's label rate");
        for pesticide in ["fungicide", "herbicide", "insecticide", "miticide"] {
            assert!(p.contains(pesticide), "names {pesticide} as a pesticide");
        }
    }

    // @spec PLANT-PROMPT-007
    #[test]
    fn the_prompt_expresses_nutrients_never_as_a_pesticide_material() {
        let p = SYSTEM_PROMPT.to_lowercase();
        assert!(
            p.contains("10-10-10"),
            "a fertilizer material may carry the nutrient"
        );
        assert!(
            p.contains("copper sulfate"),
            "but not a material that is also a pesticide"
        );
    }
}
