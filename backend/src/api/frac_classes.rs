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

// @spec DIS-MGMT-API-001
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

#[cfg(test)]
mod tests {
    use super::*;

    async fn get(product: Option<&str>) -> FracClassLookup {
        lookup(Query(FracClassQuery {
            product: product.map(String::from),
        }))
        .await
        .0
    }

    // @spec DIS-MGMT-API-001
    #[tokio::test]
    async fn lists_every_class_and_what_a_product_name_resolves_to() {
        let all = get(None).await;
        assert_eq!(all.options.len(), 12);
        let m5 = all
            .options
            .iter()
            .find(|o| o.id == FracClass::FracM5)
            .unwrap();
        assert_eq!(m5.label, "FRAC M5 (Chlorothalonil)");
        assert_eq!(m5.examples, &["chlorothalonil", "Daconil"]);
        assert!(all.matched.is_empty());

        let premix = get(Some("HEADWAY G")).await;
        assert_eq!(premix.matched, vec![FracClass::Frac11, FracClass::Frac3]);
        assert_eq!(premix.options.len(), 12);
        assert!(get(Some("Mystery Juice")).await.matched.is_empty());

        let json = serde_json::to_value(get(Some("Heritage")).await).unwrap();
        assert_eq!(json["matched"], serde_json::json!(["Frac11"]));
        assert_eq!(json["options"][0]["id"], "Frac1");
    }
}
