# Arrow: product-inventory

Product Inventory (the Shelf) — leaf segment `INV` under the root HLD.

## Status

**MAPPED** — overlay bootstrapped 2026-10-11 from the existing LID docs; not yet audited (`audited_sha: null`). 71 specs `[x]`, 1 `[D]`, 0 `[ ]`; 12 of 71 implemented specs carry a `@spec` annotation.

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
- [backend/src/api/products/checks_tests.rs](../../backend/src/api/products/checks_tests.rs)
- [frontend/src/components/severity.test.ts](../../frontend/src/components/severity.test.ts)
- [frontend/src/types/inventoryTargets.test.ts](../../frontend/src/types/inventoryTargets.test.ts)
- Inline `#[cfg(test)]` modules citing this prefix: `backend/src/api/recommendation_feed.rs`, `backend/src/datasources/openrouter.rs`, `backend/src/db/product_queries.rs`, `backend/src/error.rs`, `backend/src/models/inventory.rs`, `backend/src/models/product/checks.rs`

### Code
- [backend/src/api/applications/product_link.rs](../../backend/src/api/applications/product_link.rs)
- [backend/src/api/inventory.rs](../../backend/src/api/inventory.rs)
- [backend/src/api/products.rs](../../backend/src/api/products.rs)
- [backend/src/api/products/checks.rs](../../backend/src/api/products/checks.rs)
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
- [backend/src/models/shopping.rs](../../backend/src/models/shopping.rs)
- [frontend/src/components/InventoryBadge.tsx](../../frontend/src/components/InventoryBadge.tsx)
- [frontend/src/components/inventory/AddProductForm.tsx](../../frontend/src/components/inventory/AddProductForm.tsx)
- [frontend/src/components/inventory/EditProductForm.tsx](../../frontend/src/components/inventory/EditProductForm.tsx)
- [frontend/src/components/inventory/LogSuggestions.tsx](../../frontend/src/components/inventory/LogSuggestions.tsx)
- [frontend/src/components/inventory/ProductRow.tsx](../../frontend/src/components/inventory/ProductRow.tsx)
- [frontend/src/components/inventory/ShoppingListPanel.tsx](../../frontend/src/components/inventory/ShoppingListPanel.tsx)
- [frontend/src/components/inventory/StockBadge.tsx](../../frontend/src/components/inventory/StockBadge.tsx)
- [frontend/src/components/inventory/useProductActions.ts](../../frontend/src/components/inventory/useProductActions.ts)
- [frontend/src/components/severity.ts](../../frontend/src/components/severity.ts)
- [frontend/src/pages/Inventory.tsx](../../frontend/src/pages/Inventory.tsx)
- [frontend/src/types/inventory.ts](../../frontend/src/types/inventory.ts)
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
| SHELF | INV-SHELF-001 – INV-SHELF-011 | 11 | 0 | 0 | 0 | 0 |
| API | INV-API-001 – INV-API-014 | 14 | 0 | 0 | 4 | 4 |
| LLM | INV-LLM-001 – INV-LLM-011 | 11 | 0 | 0 | 2 | 2 |
| RATE | INV-RATE-001 – INV-RATE-002 | 2 | 0 | 0 | 0 | 0 |
| LOG | INV-LOG-001 – INV-LOG-004 | 4 | 0 | 0 | 0 | 0 |
| NEED | INV-NEED-001 – INV-NEED-006 | 5 | 1 | 0 | 1 | 1 |
| ENRICH | INV-ENRICH-001 – INV-ENRICH-005 | 5 | 0 | 0 | 2 | 2 |
| SHOP | INV-SHOP-001 – INV-SHOP-006 | 6 | 0 | 0 | 0 | 0 |
| UI | INV-UI-001 – INV-UI-013 | 13 | 0 | 0 | 3 | 2 |
| **Total** | | **71** | **1** | **0** | **12** | **11** |

**Summary:** 71 of 71 active specs implemented; 1 deferred; 11 implemented specs cited by a test, 59 cited nowhere.

## Key Findings

1. **59 implemented specs have no `@spec` annotation** — the specs were written to describe existing code and never linked back: INV-SHELF-001/002/003/004/005/006/007/008/009/010/011; INV-API-001/002/003/004/005/006/007/008/009/010; INV-LLM-001/002/003/004/005/006/007/008/009; INV-RATE-001/002; INV-LOG-001/002/003/004; INV-NEED-001/002/003/004; INV-ENRICH-001/003/004; INV-SHOP-001/002/003/004/005/006; INV-UI-001/002/003/004/005/006/008/009/010/012.
2. **1 specs are annotated in code but cited by no test** — INV-UI-013.
3. **Deferred:** INV-NEED-006.

## Work Required

### Must Fix
- None.

### Should Fix
1. Add `@spec` annotations (entry point + exercising test) for the 59 unlinked specs.
2. Annotate a test for the 1 code-only specs.

### Nice to Have
- Audit each `[x]` against the code and move the segment to `AUDITED`.
