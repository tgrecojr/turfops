use crate::db::queries::enum_to_db_string;
use crate::error::{Result, TurfOpsError};
use crate::models::product::*;
use crate::models::FracClass;
use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use sqlx::PgPool;
use tracing::warn;

pub async fn list_products_for_profile(
    pool: &PgPool,
    profile_id: i64,
    category: Option<ProductCategory>,
    include_archived: bool,
) -> Result<Vec<Product>> {
    let rows = sqlx::query_as::<_, ProductRow>(
        r#"SELECT id, lawn_profile_id, name, brand, category, form, stock_status,
           label_rate_per_1000sqft, label_rate_unit, nitrogen_pct, phosphorus_pct, potassium_pct,
           frac_classes, herbicide_timing, targets, amendment_kind,
           profile, profile_generated_at, profile_model, notes, archived, created_at, updated_at
           FROM products
           WHERE lawn_profile_id = $1
             AND ($2::text IS NULL OR category = $2)
             AND ($3 OR NOT archived)
           ORDER BY category, lower(name)"#,
    )
    .bind(profile_id)
    .bind(category.map(|c| c.as_str()))
    .bind(include_archived)
    .fetch_all(pool)
    .await?;

    rows.into_iter().map(ProductRow::into_product).collect()
}

pub async fn get_product(pool: &PgPool, id: i64) -> Result<Option<Product>> {
    let row = sqlx::query_as::<_, ProductRow>(
        r#"SELECT id, lawn_profile_id, name, brand, category, form, stock_status,
           label_rate_per_1000sqft, label_rate_unit, nitrogen_pct, phosphorus_pct, potassium_pct,
           frac_classes, herbicide_timing, targets, amendment_kind,
           profile, profile_generated_at, profile_model, notes, archived, created_at, updated_at
           FROM products WHERE id = $1"#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    row.map(ProductRow::into_product).transpose()
}

pub async fn create_product(pool: &PgPool, product: &Product) -> Result<i64> {
    let f = &product.facts;
    let id = sqlx::query_scalar::<_, i64>(
        r#"
        INSERT INTO products
            (lawn_profile_id, name, brand, category, form, stock_status,
             label_rate_per_1000sqft, label_rate_unit, nitrogen_pct, phosphorus_pct, potassium_pct,
             frac_classes, herbicide_timing, targets, amendment_kind,
             profile, profile_generated_at, profile_model, notes)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19)
        RETURNING id
        "#,
    )
    .bind(product.lawn_profile_id)
    .bind(product.name.trim())
    .bind(product.brand.as_deref().map(str::trim))
    .bind(f.category.as_str())
    .bind(f.form.as_str())
    .bind(product.stock_status.as_str())
    .bind(f.label_rate_per_1000sqft)
    .bind(f.label_rate_unit.map(|u| u.as_str()))
    .bind(f.nitrogen_pct)
    .bind(f.phosphorus_pct)
    .bind(f.potassium_pct)
    .bind(enums_to_db(&f.frac_classes)?)
    .bind(f.herbicide_timing.map(|t| t.as_str()))
    .bind(enums_to_db(&f.targets)?)
    .bind(f.amendment_kind.map(|k| k.as_str()))
    .bind(profile_json(product.profile.as_ref())?)
    .bind(product.profile_generated_at)
    .bind(&product.profile_model)
    .bind(&product.notes)
    .fetch_one(pool)
    .await?;
    Ok(id)
}

/// Everything the user owns on a product. The profile is untouched.
pub struct ProductUpdate {
    pub name: String,
    pub brand: Option<String>,
    pub stock_status: StockStatus,
    pub facts: ProductFacts,
    pub notes: Option<String>,
    pub archived: bool,
}

pub async fn update_product(pool: &PgPool, id: i64, update: &ProductUpdate) -> Result<()> {
    let f = &update.facts;
    let result = sqlx::query(
        r#"
        UPDATE products SET
            name = $1, brand = $2, category = $3, form = $4, stock_status = $5,
            label_rate_per_1000sqft = $6, label_rate_unit = $7,
            nitrogen_pct = $8, phosphorus_pct = $9, potassium_pct = $10,
            frac_classes = $11, herbicide_timing = $12, targets = $13, amendment_kind = $14,
            notes = $15, archived = $16, updated_at = NOW()
        WHERE id = $17
        "#,
    )
    .bind(update.name.trim())
    .bind(update.brand.as_deref().map(str::trim))
    .bind(f.category.as_str())
    .bind(f.form.as_str())
    .bind(update.stock_status.as_str())
    .bind(f.label_rate_per_1000sqft)
    .bind(f.label_rate_unit.map(|u| u.as_str()))
    .bind(f.nitrogen_pct)
    .bind(f.phosphorus_pct)
    .bind(f.potassium_pct)
    .bind(enums_to_db(&f.frac_classes)?)
    .bind(f.herbicide_timing.map(|t| t.as_str()))
    .bind(enums_to_db(&f.targets)?)
    .bind(f.amendment_kind.map(|k| k.as_str()))
    .bind(&update.notes)
    .bind(update.archived)
    .bind(id)
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(TurfOpsError::NotFound(format!("Product {} not found", id)));
    }
    Ok(())
}

pub async fn update_product_profile(
    pool: &PgPool,
    id: i64,
    profile: &ProductProfile,
    model: &str,
) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE products SET
            profile = $1, profile_generated_at = NOW(), profile_model = $2, updated_at = NOW()
        WHERE id = $3
        "#,
    )
    .bind(profile_json(Some(profile))?)
    .bind(model)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_product(pool: &PgPool, id: i64) -> Result<()> {
    sqlx::query("DELETE FROM products WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

fn enums_to_db<T: serde::Serialize + Copy>(list: &[T]) -> Result<Vec<String>> {
    list.iter().map(|v| enum_to_db_string(*v)).collect()
}

fn profile_json(profile: Option<&ProductProfile>) -> Result<Option<serde_json::Value>> {
    profile
        .map(serde_json::to_value)
        .transpose()
        .map_err(|e| TurfOpsError::InvalidData(format!("Profile serialization: {}", e)))
}

/// A stored variant name → enum. Unknown values are dropped with a warning so one bad
/// row does not take the whole list down.
fn parse_enum<T: DeserializeOwned>(field: &str, code: &str) -> Option<T> {
    let parsed = serde_json::from_value(serde_json::Value::String(code.to_string())).ok();
    if parsed.is_none() {
        warn!(field, value = %code, "Unknown enum value in products table, ignoring");
    }
    parsed
}

fn parse_enums<T: DeserializeOwned>(field: &str, codes: Vec<String>) -> Vec<T> {
    codes.iter().filter_map(|c| parse_enum(field, c)).collect()
}

#[derive(sqlx::FromRow)]
struct ProductRow {
    id: i64,
    lawn_profile_id: i64,
    name: String,
    brand: Option<String>,
    category: String,
    form: String,
    stock_status: String,
    label_rate_per_1000sqft: Option<f64>,
    label_rate_unit: Option<String>,
    nitrogen_pct: Option<f64>,
    phosphorus_pct: Option<f64>,
    potassium_pct: Option<f64>,
    frac_classes: Vec<String>,
    herbicide_timing: Option<String>,
    targets: Vec<String>,
    amendment_kind: Option<String>,
    profile: Option<serde_json::Value>,
    profile_generated_at: Option<DateTime<Utc>>,
    profile_model: Option<String>,
    notes: Option<String>,
    archived: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl ProductRow {
    fn into_product(self) -> Result<Product> {
        let required = |field: &str, code: &str| {
            TurfOpsError::InvalidData(format!(
                "Product {} has unknown {}: {}",
                self.id, field, code
            ))
        };
        let category = parse_enum("category", &self.category)
            .ok_or_else(|| required("category", &self.category))?;
        let form = parse_enum("form", &self.form).ok_or_else(|| required("form", &self.form))?;
        let stock_status =
            parse_enum("stock_status", &self.stock_status).unwrap_or(StockStatus::InStock);
        let profile = self
            .profile
            .map(serde_json::from_value::<ProductProfile>)
            .transpose()
            .map_err(|e| {
                TurfOpsError::InvalidData(format!(
                    "Product {} has invalid profile JSON: {}",
                    self.id, e
                ))
            })?;

        Ok(Product {
            id: Some(self.id),
            lawn_profile_id: self.lawn_profile_id,
            name: self.name,
            brand: self.brand,
            stock_status,
            facts: ProductFacts {
                category,
                form,
                label_rate_per_1000sqft: self.label_rate_per_1000sqft,
                label_rate_unit: self
                    .label_rate_unit
                    .and_then(|u| parse_enum("label_rate_unit", &u)),
                nitrogen_pct: self.nitrogen_pct,
                phosphorus_pct: self.phosphorus_pct,
                potassium_pct: self.potassium_pct,
                frac_classes: parse_enums::<FracClass>("frac_classes", self.frac_classes),
                herbicide_timing: self
                    .herbicide_timing
                    .and_then(|t| parse_enum("herbicide_timing", &t)),
                targets: parse_enums("targets", self.targets),
                amendment_kind: self
                    .amendment_kind
                    .and_then(|k| parse_enum("amendment_kind", &k)),
            },
            profile,
            profile_generated_at: self.profile_generated_at,
            profile_model: self.profile_model,
            notes: self.notes,
            archived: self.archived,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}
