use crate::models::{frac_classes_for_product, FracClass};
use axum::extract::Query;
use axum::Json;
use serde::{Deserialize, Serialize};

const ALL_CLASSES: [FracClass; 12] = [
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

#[derive(Debug, Deserialize)]
pub struct FracClassQuery {
    pub product: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FracClassOption {
    pub id: FracClass,
    pub label: &'static str,
    pub examples: &'static [&'static str],
}

#[derive(Debug, Serialize)]
pub struct FracClassLookup {
    /// Every class the application form can record.
    pub options: Vec<FracClassOption>,
    /// Classes the `product` name resolves to. Empty = not recognised, and the form asks
    /// the user to pick from the label so the disease models can count the application.
    pub matched: Vec<FracClass>,
}

/// GET /api/v1/frac-classes?product=<name>
pub async fn lookup(Query(params): Query<FracClassQuery>) -> Json<FracClassLookup> {
    Json(FracClassLookup {
        options: ALL_CLASSES
            .iter()
            .map(|class| FracClassOption {
                id: *class,
                label: class.as_str(),
                examples: class.common_products(),
            })
            .collect(),
        matched: params
            .product
            .as_deref()
            .map(frac_classes_for_product)
            .unwrap_or_default(),
    })
}
