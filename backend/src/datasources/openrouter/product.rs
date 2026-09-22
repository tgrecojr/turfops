//! Prompt, strict JSON schema and request type for identifying a lawn-care product.
//! The schema has no rate interval or timing fields on purpose: the label governs. The one
//! rate it may return is fenced as a *suggestion* the user confirms (see the plan doc).
use super::OpenRouterClient;
use crate::error::Result;
use crate::models::product::*;
use crate::models::FracClass;
use serde::Serialize;
use serde_json::{json, Value};

#[derive(Debug, Serialize)]
pub struct ProductProfileRequest<'a> {
    pub name: &'a str,
    pub category_hint: Option<ProductCategory>,
    pub grass_type: &'a str,
    pub usda_zone: &'a str,
}

impl OpenRouterClient {
    /// Identify a product and describe it in the structured `ProductProfile` shape.
    pub async fn generate_product_profile(
        &self,
        req: ProductProfileRequest<'_>,
    ) -> Result<ProductProfile> {
        let user_prompt = format!(
            "Identify this lawn-care product and describe it.\n\
             Product: {}\n\
             Category (user-reported): {}\n\
             Lawn grass type: {}\n\
             USDA hardiness zone: {}\n",
            req.name,
            req.category_hint
                .map(|c| c.as_str())
                .unwrap_or("unknown — you decide"),
            req.grass_type,
            req.usda_zone,
        );
        self.complete_json(
            PRODUCT_SYSTEM_PROMPT,
            user_prompt,
            "product_profile",
            product_json_schema(),
            0.1,
        )
        .await
    }
}

pub(super) const PRODUCT_SYSTEM_PROMPT: &str = "You are cataloguing lawn-care products for a \
HOMEOWNER's shelf: fertilizers, nutrient supplements, fungicides, herbicides, insect control, \
grass seed, soil amendments and surfactants. The user gives a product name as printed on the \
bag or bottle, sometimes with a brand, plus the category they think it is.\n\n\
Your job:\n\
1. Identify the product. Prefer the exact retail product; if several products share the name, \
   pick the most common residential one and lower the confidence. If you do not recognise it, \
   return confidence \"Low\", category \"Other\" (or the user's category), and say so in the \
   summary — never invent a formulation.\n\
2. Report only what the label states: active ingredients with percentages, the guaranteed \
   analysis N-P-K for fertilizers, FRAC codes for fungicides (only codes from the schema's \
   list; leave the list empty if unsure), pre-/post-emergent for herbicides, and the targets \
   it controls, supplies or contains (only values from the schema's list).\n\
3. suggested_label_rate: fill it ONLY when the product's lawn rate per 1,000 sq ft is \
   commonly published on its label and you are confident of it; otherwise null. Never \
   estimate a rate, and never give application intervals or timing anywhere in the output.\n\
4. cautions: at most 4 label-class facts a homeowner must know (e.g. \"not labeled for \
   residential lawns\", \"do not apply above 85 °F\", \"water in within 24 h\", \"keep off \
   ornamentals\"). No general safety boilerplate.\n\
5. summary: 1–2 plain sentences on what it is and what it is for, tuned to the grass type \
   and zone given if relevant (e.g. a product not suited to cool-season turf).\n\n\
Output MUST conform to the provided JSON schema exactly. Do not include any prose outside \
the JSON.";

fn names<T: Serialize>(all: &[T]) -> Vec<Value> {
    all.iter()
        .map(|v| serde_json::to_value(v).expect("string enum"))
        .collect()
}

fn enum_schema<T: Serialize>(all: &[T]) -> Value {
    json!({ "type": "string", "enum": names(all) })
}

fn nullable(schema: Value) -> Value {
    json!({ "anyOf": [schema, { "type": "null" }] })
}

fn nullable_number() -> Value {
    json!({ "type": ["number", "null"] })
}

pub(super) fn product_json_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": [
            "identified_name", "manufacturer", "category", "form", "active_ingredients",
            "npk", "frac_classes", "herbicide_timing", "targets", "amendment_kind",
            "mobility", "nitrogen_release", "suitable_for", "suggested_label_rate",
            "summary", "cautions", "confidence"
        ],
        "properties": {
            "identified_name": { "type": "string" },
            "manufacturer": { "type": ["string", "null"] },
            "category": enum_schema(ProductCategory::ALL),
            "form": enum_schema(ProductForm::ALL),
            "active_ingredients": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["name", "pct"],
                    "properties": {
                        "name": { "type": "string" },
                        "pct": nullable_number()
                    }
                }
            },
            "npk": nullable(json!({
                "type": "object",
                "additionalProperties": false,
                "required": ["n", "p", "k"],
                "properties": {
                    "n": { "type": "number" },
                    "p": { "type": "number" },
                    "k": { "type": "number" }
                }
            })),
            "frac_classes": { "type": "array", "items": enum_schema(ALL_FRAC) },
            "herbicide_timing": nullable(enum_schema(HerbicideTiming::ALL)),
            "targets": { "type": "array", "items": enum_schema(ProductTarget::ALL) },
            "amendment_kind": nullable(enum_schema(AmendmentKind::ALL)),
            "mobility": enum_schema(Mobility::ALL),
            "nitrogen_release": enum_schema(NitrogenRelease::ALL),
            "suitable_for": enum_schema(SuitableFor::ALL),
            "suggested_label_rate": nullable(json!({
                "type": "object",
                "additionalProperties": false,
                "required": ["amount", "unit"],
                "properties": {
                    "amount": { "type": "number" },
                    "unit": enum_schema(RateUnit::ALL)
                }
            })),
            "summary": { "type": "string" },
            "cautions": { "type": "array", "items": { "type": "string" } },
            "confidence": { "type": "string", "enum": ["High", "Medium", "Low"] }
        }
    })
}

const ALL_FRAC: &[FracClass] = &[
    FracClass::Frac1,
    FracClass::Frac3,
    FracClass::Frac4,
    FracClass::Frac7,
    FracClass::Frac11,
    FracClass::Frac12,
    FracClass::Frac14,
    FracClass::Frac21,
    FracClass::Frac28,
    FracClass::FracP07,
    FracClass::FracM3,
    FracClass::FracM5,
];

#[cfg(test)]
mod tests {
    use super::super::tests::sample_config;
    use super::*;
    use crate::error::TurfOpsError;

    #[test]
    fn schema_requires_every_profile_field_and_has_no_rate_interval() {
        let schema = product_json_schema();
        let required: Vec<&str> = schema["required"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str())
            .collect();
        let props = schema["properties"].as_object().unwrap();
        assert_eq!(
            required.len(),
            props.len(),
            "strict mode: all props required"
        );
        for key in [
            "frac_classes",
            "targets",
            "suggested_label_rate",
            "confidence",
        ] {
            assert!(required.contains(&key));
        }
        assert!(!props.contains_key("interval"));
        assert!(!props.contains_key("timing"));
        assert_eq!(
            props["targets"]["items"]["enum"].as_array().unwrap().len(),
            ProductTarget::ALL.len()
        );
    }

    #[test]
    fn schema_round_trips_through_the_model() {
        // A reply that uses every enum must deserialize into ProductProfile.
        let reply = json!({
            "identified_name": "Scotts Turf Builder",
            "manufacturer": "Scotts",
            "category": "Fertilizer",
            "form": "Granular",
            "active_ingredients": [{ "name": "urea", "pct": null }],
            "npk": { "n": 32.0, "p": 0.0, "k": 4.0 },
            "frac_classes": [],
            "herbicide_timing": null,
            "targets": ["Iron"],
            "amendment_kind": null,
            "mobility": "NotApplicable",
            "nitrogen_release": "Mixed",
            "suitable_for": "Turf",
            "suggested_label_rate": { "amount": 2.9, "unit": "Lb" },
            "summary": "A 32-0-4 lawn fertilizer.",
            "cautions": [],
            "confidence": "High"
        });
        let profile: ProductProfile = serde_json::from_value(reply).unwrap();
        assert_eq!(profile.npk.unwrap().n, 32.0);
        assert_eq!(profile.suggested_label_rate.unwrap().unit, RateUnit::Lb);
    }

    #[test]
    fn disabled_client_errors() {
        let mut cfg = sample_config();
        cfg.enabled = false;
        let client = OpenRouterClient::new(cfg);
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let result = rt.block_on(client.generate_product_profile(ProductProfileRequest {
            name: "Heritage G",
            category_hint: None,
            grass_type: "Tall Fescue",
            usda_zone: "7a",
        }));
        assert!(matches!(
            result,
            Err(TurfOpsError::DataSourceUnavailable(_))
        ));
    }
}
