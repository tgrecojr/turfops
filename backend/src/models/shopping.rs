//! The shopping list: what to buy (needs nothing on the shelf covers) and what to restock
//! (Low / Out products that an active recommendation calls for).
use super::product::ProductCategory;
use super::{Severity, ShelfProduct};
use chrono::{DateTime, Utc};
use serde::Serialize;

/// Ordered so a list sorts Buy before Restock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum ShoppingKind {
    /// Nothing on the shelf covers the need.
    Buy,
    /// A product that fits is on the shelf but marked Low or Out.
    Restock,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ShoppingItem {
    pub kind: ShoppingKind,
    /// The need's label, or the product's name for a restock.
    pub label: String,
    pub categories: Vec<ProductCategory>,
    /// The Low / Out product, for restocks.
    pub product: Option<ShelfProduct>,
    /// Titles of the recommendations that call for it.
    pub reasons: Vec<String>,
    /// The most urgent of those recommendations.
    pub urgency: Severity,
}

#[derive(Debug, Clone, Serialize)]
pub struct ShoppingList {
    pub generated_at: DateTime<Utc>,
    pub items: Vec<ShoppingItem>,
}
