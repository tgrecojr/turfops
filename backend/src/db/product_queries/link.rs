//! Bridging the application log and the shelf: names logged but never catalogued, and
//! back-linking past rows to a product by name.
use crate::error::Result;
use chrono::NaiveDate;
use serde::Serialize;
use sqlx::PgPool;

/// A product name from the log that matches no product (case-insensitive).
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct LoggedProductSuggestion {
    pub name: String,
    pub uses: i64,
    pub last_used: NaiveDate,
    pub application_types: Vec<String>,
}

pub async fn logged_product_suggestions(
    pool: &PgPool,
    profile_id: i64,
) -> Result<Vec<LoggedProductSuggestion>> {
    let rows = sqlx::query_as::<_, LoggedProductSuggestion>(
        r#"SELECT (array_agg(a.product_name ORDER BY a.application_date DESC))[1] AS name,
                  count(*) AS uses,
                  max(a.application_date) AS last_used,
                  array_agg(DISTINCT a.application_type) AS application_types
           FROM applications a
           WHERE a.lawn_profile_id = $1
             AND a.product_id IS NULL
             AND a.product_name IS NOT NULL
             AND btrim(a.product_name) <> ''
             AND NOT EXISTS (
                 SELECT 1 FROM products p
                 WHERE p.lawn_profile_id = a.lawn_profile_id
                   AND lower(btrim(p.name)) = lower(btrim(a.product_name))
             )
           GROUP BY lower(btrim(a.product_name))
           ORDER BY uses DESC, last_used DESC"#,
    )
    .bind(profile_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Set `product_id` on every unlinked application whose name matches the product's.
/// Returns how many rows were linked.
pub async fn link_applications_by_name(
    pool: &PgPool,
    product_id: i64,
    profile_id: i64,
    name: &str,
) -> Result<u64> {
    let result = sqlx::query(
        r#"UPDATE applications SET product_id = $1
           WHERE lawn_profile_id = $2
             AND product_id IS NULL
             AND lower(btrim(product_name)) = lower(btrim($3))"#,
    )
    .bind(product_id)
    .bind(profile_id)
    .bind(name)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}
