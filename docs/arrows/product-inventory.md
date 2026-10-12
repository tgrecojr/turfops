# Arrow: product-inventory

Product Inventory (the Shelf) — leaf segment `INV` under the root HLD.

## Status

**OK** — audited 2026-10-12 (git SHA `d99ce09`): every implemented spec was checked against the code while linking it. 71 of 71 implemented specs are linked to both code and a test.

## References

### HLD
- [docs/high-level-design.md](../../docs/high-level-design.md) § System Design — component "Product inventory"

### LLD
- [docs/intent/product-inventory/product-inventory-design.md](../../docs/intent/product-inventory/product-inventory-design.md)

### EARS
- [docs/intent/product-inventory/product-inventory-specs.md](../../docs/intent/product-inventory/product-inventory-specs.md) (72 specs, prefix `INV`)

### Related docs
- [docs/agronomy-methods.md](../../docs/agronomy-methods.md)
- [docs/plans/product-inventory.md](../../docs/plans/product-inventory.md)

### Tests
- [backend/src/api/products/assistant_db_tests.rs](../../backend/src/api/products/assistant_db_tests.rs)
- [backend/src/api/products/checks_tests.rs](../../backend/src/api/products/checks_tests.rs)
- [backend/src/api/products/crud_db_tests.rs](../../backend/src/api/products/crud_db_tests.rs)
- [backend/src/api/products/edit_db_tests.rs](../../backend/src/api/products/edit_db_tests.rs)
- [backend/src/api/products/link_db_tests.rs](../../backend/src/api/products/link_db_tests.rs)
- [backend/src/api/products/missing_category_db_tests.rs](../../backend/src/api/products/missing_category_db_tests.rs)
- [backend/src/datasources/openrouter/product/schema_tests.rs](../../backend/src/datasources/openrouter/product/schema_tests.rs)
- [backend/src/db/product_queries/row_tests.rs](../../backend/src/db/product_queries/row_tests.rs)
- [backend/src/logic/inventory/enrich_tests.rs](../../backend/src/logic/inventory/enrich_tests.rs)
- [backend/src/logic/inventory/shopping/spec_tests.rs](../../backend/src/logic/inventory/shopping/spec_tests.rs)
- [backend/src/models/inventory/need_tests.rs](../../backend/src/models/inventory/need_tests.rs)
- [backend/src/models/product/accepts_tests.rs](../../backend/src/models/product/accepts_tests.rs)
- [backend/src/models/product/vocab_tests.rs](../../backend/src/models/product/vocab_tests.rs)
- [frontend/src/api/productTimeouts.test.ts](../../frontend/src/api/productTimeouts.test.ts)
- [frontend/src/components/InventoryBadge.test.tsx](../../frontend/src/components/InventoryBadge.test.tsx)
- [frontend/src/components/inventory/AddProductForm.test.tsx](../../frontend/src/components/inventory/AddProductForm.test.tsx)
- [frontend/src/components/inventory/LogSuggestions.test.tsx](../../frontend/src/components/inventory/LogSuggestions.test.tsx)
- [frontend/src/components/inventory/ProductDetails.test.tsx](../../frontend/src/components/inventory/ProductDetails.test.tsx)
- [frontend/src/components/inventory/ProductRow.test.tsx](../../frontend/src/components/inventory/ProductRow.test.tsx)
- [frontend/src/components/inventory/ShoppingListPanel.test.tsx](../../frontend/src/components/inventory/ShoppingListPanel.test.tsx)
- [frontend/src/components/inventory/StockBadge.test.tsx](../../frontend/src/components/inventory/StockBadge.test.tsx)
- [frontend/src/components/severity.test.ts](../../frontend/src/components/severity.test.ts)
- [frontend/src/pages/Inventory.test.tsx](../../frontend/src/pages/Inventory.test.tsx)
- [frontend/src/pages/InventoryActions.test.tsx](../../frontend/src/pages/InventoryActions.test.tsx)
- [frontend/src/types/inventoryCompat.test.ts](../../frontend/src/types/inventoryCompat.test.ts)
- [frontend/src/types/inventoryTargets.test.ts](../../frontend/src/types/inventoryTargets.test.ts)
- Inline `#[cfg(test)]` modules citing this prefix: `backend/src/api/recommendation_feed.rs`, `backend/src/datasources/openrouter.rs`, `backend/src/datasources/openrouter/product.rs`, `backend/src/db/product_queries.rs`, `backend/src/error.rs`, `backend/src/logic/inventory.rs`, `backend/src/logic/inventory/shopping.rs`, `backend/src/models/inventory.rs`, `backend/src/models/product.rs`, `backend/src/models/product/checks.rs`, `backend/src/models/product/enums.rs`

### Code
- [backend/src/api/applications/product_link.rs](../../backend/src/api/applications/product_link.rs)
- [backend/src/api/inventory.rs](../../backend/src/api/inventory.rs)
- [backend/src/api/products.rs](../../backend/src/api/products.rs)
- [backend/src/api/products/checks.rs](../../backend/src/api/products/checks.rs)
- [backend/src/api/products/link.rs](../../backend/src/api/products/link.rs)
- [backend/src/api/products/refresh.rs](../../backend/src/api/products/refresh.rs)
- [backend/src/api/recommendation_feed.rs](../../backend/src/api/recommendation_feed.rs)
- [backend/src/datasources/openrouter.rs](../../backend/src/datasources/openrouter.rs)
- [backend/src/datasources/openrouter/product.rs](../../backend/src/datasources/openrouter/product.rs)
- [backend/src/db/migrations/20260922000000_products.sql](../../backend/src/db/migrations/20260922000000_products.sql)
- [backend/src/db/migrations/20260922000100_application_product.sql](../../backend/src/db/migrations/20260922000100_application_product.sql)
- [backend/src/db/migrations/20260923000000_product_facts_scoped.sql](../../backend/src/db/migrations/20260923000000_product_facts_scoped.sql)
- [backend/src/db/product_queries.rs](../../backend/src/db/product_queries.rs)
- [backend/src/error.rs](../../backend/src/error.rs)
- [backend/src/logic/inventory.rs](../../backend/src/logic/inventory.rs)
- [backend/src/logic/inventory/shopping.rs](../../backend/src/logic/inventory/shopping.rs)
- [backend/src/models/inventory.rs](../../backend/src/models/inventory.rs)
- [backend/src/models/product.rs](../../backend/src/models/product.rs)
- [backend/src/models/product/checks.rs](../../backend/src/models/product/checks.rs)
- [backend/src/models/product/enums.rs](../../backend/src/models/product/enums.rs)
- [backend/src/models/product/profile.rs](../../backend/src/models/product/profile.rs)
- [backend/src/models/shopping.rs](../../backend/src/models/shopping.rs)
- [frontend/src/api/client.ts](../../frontend/src/api/client.ts)
- [frontend/src/components/InventoryBadge.tsx](../../frontend/src/components/InventoryBadge.tsx)
- [frontend/src/components/inventory/AddProductForm.tsx](../../frontend/src/components/inventory/AddProductForm.tsx)
- [frontend/src/components/inventory/EditProductForm.tsx](../../frontend/src/components/inventory/EditProductForm.tsx)
- [frontend/src/components/inventory/LogSuggestions.tsx](../../frontend/src/components/inventory/LogSuggestions.tsx)
- [frontend/src/components/inventory/ProductDetails.tsx](../../frontend/src/components/inventory/ProductDetails.tsx)
- [frontend/src/components/inventory/ProductRow.tsx](../../frontend/src/components/inventory/ProductRow.tsx)
- [frontend/src/components/inventory/ShoppingListPanel.tsx](../../frontend/src/components/inventory/ShoppingListPanel.tsx)
- [frontend/src/components/inventory/StockBadge.tsx](../../frontend/src/components/inventory/StockBadge.tsx)
- [frontend/src/components/inventory/useProductActions.ts](../../frontend/src/components/inventory/useProductActions.ts)
- [frontend/src/components/severity.ts](../../frontend/src/components/severity.ts)
- [frontend/src/pages/Applications.tsx](../../frontend/src/pages/Applications.tsx)
- [frontend/src/pages/Inventory.tsx](../../frontend/src/pages/Inventory.tsx)
- [frontend/src/types/inventory.ts](../../frontend/src/types/inventory.ts)
- [frontend/src/types/inventoryCompat.ts](../../frontend/src/types/inventoryCompat.ts)
- [frontend/src/types/inventoryTargets.ts](../../frontend/src/types/inventoryTargets.ts)

## Architecture

**Purpose:** see the LLD's Context and Design Philosophy; this page carries pointers and coverage only.

**Spec facets:**
1. `INV-SHELF` — the product record, vocabularies, category rules and scoping
2. `INV-API` — product endpoints and their errors
3. `INV-LLM` — assistant profile, precedence and refresh diff
4. `INV-RATE` — the product label-rate rule
5. `INV-LOG` — log suggestions and linking past applications
6. `INV-NEED` — product needs and matching
7. `INV-ENRICH` — inventory status and the feed's inventory pass
8. `INV-SHOP` — shopping list
9. `INV-UI` — Inventory page, stock pill, inventory badge, shopping panel

## Spec Coverage

Derived from the spec file and a `@spec` scan; regenerate, don't hand-edit.

| Facet | Spec IDs | Implemented | Deferred | Gaps | `@spec` in code | `@spec` in tests |
|---|---|---|---|---|---|---|
| SHELF | INV-SHELF-001 – INV-SHELF-011 | 11 | 0 | 0 | 11 | 11 |
| API | INV-API-001 – INV-API-014 | 14 | 0 | 0 | 14 | 14 |
| LLM | INV-LLM-001 – INV-LLM-011 | 11 | 0 | 0 | 11 | 11 |
| RATE | INV-RATE-001 – INV-RATE-002 | 2 | 0 | 0 | 2 | 2 |
| LOG | INV-LOG-001 – INV-LOG-004 | 4 | 0 | 0 | 4 | 4 |
| NEED | INV-NEED-001 – INV-NEED-006 | 5 | 1 | 0 | 5 | 5 |
| ENRICH | INV-ENRICH-001 – INV-ENRICH-005 | 5 | 0 | 0 | 5 | 5 |
| SHOP | INV-SHOP-001 – INV-SHOP-006 | 6 | 0 | 0 | 6 | 6 |
| UI | INV-UI-001 – INV-UI-013 | 13 | 0 | 0 | 13 | 13 |
| **Total** | | **71** | **1** | **0** | **71** | **71** |

**Summary:** 71 of 71 active specs implemented; 1 deferred; 71 implemented specs cited by a test, 0 cited nowhere.

## Key Findings

1. **Deferred:** INV-NEED-006.

## Work Required

### Must Fix
- None.

### Should Fix
- None.

### Nice to Have
- None.
