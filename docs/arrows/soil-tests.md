# Arrow: soil-tests

Soil Tests — leaf segment `SOIL` under the root HLD.

## Status

**OK** — audited 2026-10-12 (git SHA `2bf061e`): every implemented spec was checked against the code while linking it. 64 of 64 implemented specs are linked to both code and a test.

## References

### HLD
- [docs/high-level-design.md](../../docs/high-level-design.md) § System Design — component "Soil tests"

### LLD
- [docs/intent/soil-tests/soil-tests-design.md](../../docs/intent/soil-tests/soil-tests-design.md)

### EARS
- [docs/intent/soil-tests/soil-tests-specs.md](../../docs/intent/soil-tests/soil-tests-specs.md) (72 specs, prefix `SOIL`)

### Related docs
- [docs/plans/product-inventory.md](../../docs/plans/product-inventory.md)

### Tests
- [backend/src/api/soil_tests/db_data_tests.rs](../../backend/src/api/soil_tests/db_data_tests.rs)
- [backend/src/api/soil_tests/db_tests.rs](../../backend/src/api/soil_tests/db_tests.rs)
- [backend/src/api/soil_tests/tests.rs](../../backend/src/api/soil_tests/tests.rs)
- [backend/src/logic/soil_test_recommendations/advice_tests.rs](../../backend/src/logic/soil_test_recommendations/advice_tests.rs)
- [backend/src/logic/soil_test_recommendations/gap_tests.rs](../../backend/src/logic/soil_test_recommendations/gap_tests.rs)
- [backend/src/logic/soil_test_recommendations/npk_tests.rs](../../backend/src/logic/soil_test_recommendations/npk_tests.rs)
- [backend/src/logic/soil_test_recommendations/ph_tests.rs](../../backend/src/logic/soil_test_recommendations/ph_tests.rs)
- [backend/src/logic/soil_test_recommendations/tests.rs](../../backend/src/logic/soil_test_recommendations/tests.rs)
- [frontend/src/components/soilTests/AdvicePanel.test.tsx](../../frontend/src/components/soilTests/AdvicePanel.test.tsx)
- [frontend/src/components/soilTests/display.test.ts](../../frontend/src/components/soilTests/display.test.ts)
- [frontend/src/pages/SoilTests.route.test.tsx](../../frontend/src/pages/SoilTests.route.test.tsx)
- [frontend/src/pages/SoilTests.test.tsx](../../frontend/src/pages/SoilTests.test.tsx)

### Code
- [backend/src/api/soil_tests.rs](../../backend/src/api/soil_tests.rs)
- [backend/src/db/migrations/20240101000007_soil_tests.sql](../../backend/src/db/migrations/20240101000007_soil_tests.sql)
- [backend/src/db/soil_test_queries.rs](../../backend/src/db/soil_test_queries.rs)
- [backend/src/logic/rules/nitrogen_guard.rs](../../backend/src/logic/rules/nitrogen_guard.rs)
- [backend/src/logic/soil_test_recommendations.rs](../../backend/src/logic/soil_test_recommendations.rs)
- [backend/src/logic/soil_test_recommendations/ratio.rs](../../backend/src/logic/soil_test_recommendations/ratio.rs)
- [backend/src/logic/soil_test_thresholds.rs](../../backend/src/logic/soil_test_thresholds.rs)
- [backend/src/main.rs](../../backend/src/main.rs)
- [backend/src/mcp/instructions.rs](../../backend/src/mcp/instructions.rs)
- [backend/src/mcp/tools/log.rs](../../backend/src/mcp/tools/log.rs)
- [backend/src/models/inventory.rs](../../backend/src/models/inventory.rs)
- [backend/src/models/nitrogen_budget.rs](../../backend/src/models/nitrogen_budget.rs)
- [backend/src/models/product/profile.rs](../../backend/src/models/product/profile.rs)
- [backend/src/models/recommendation_state.rs](../../backend/src/models/recommendation_state.rs)
- [backend/src/models/soil_test.rs](../../backend/src/models/soil_test.rs)
- [frontend/src/App.tsx](../../frontend/src/App.tsx)
- [frontend/src/api/client.ts](../../frontend/src/api/client.ts)
- [frontend/src/components/soilTests](../../frontend/src/components/soilTests)
- [frontend/src/components/soilTests/AdvicePanel.tsx](../../frontend/src/components/soilTests/AdvicePanel.tsx)
- [frontend/src/components/soilTests/LevelBadge.tsx](../../frontend/src/components/soilTests/LevelBadge.tsx)
- [frontend/src/components/soilTests/display.ts](../../frontend/src/components/soilTests/display.ts)
- [frontend/src/pages/SoilTests.tsx](../../frontend/src/pages/SoilTests.tsx)
- [frontend/src/types/index.ts](../../frontend/src/types/index.ts)

## Architecture

**Purpose:** see the LLD's Context and Design Philosophy; this page carries pointers and coverage only.

**Spec facets:**
1. `SOIL-DATA` — stored tests and which one is latest
2. `SOIL-API` — soil-test endpoints
3. `SOIL-ADV` — rules every piece of soil-test advice follows
4. `SOIL-PH` — pH correction
5. `SOIL-NPK` — N-P-K advice and the nitrogen budget it reads
6. `SOIL-NUTR` — nutrient deficiencies
7. `SOIL-UI` — Soil Tests page
8. `SOIL-REJOIN` — rules that apply if soil-test advice returns to the recommendation feed

## Spec Coverage

Derived from the spec file and a `@spec` scan; regenerate, don't hand-edit.

| Facet | Spec IDs | Implemented | Deferred | Gaps | `@spec` in code | `@spec` in tests |
|---|---|---|---|---|---|---|
| DATA | SOIL-DATA-001 – SOIL-DATA-005 | 5 | 0 | 0 | 5 | 5 |
| API | SOIL-API-001 – SOIL-API-012 | 12 | 0 | 0 | 12 | 12 |
| ADV | SOIL-ADV-001 – SOIL-ADV-005 | 5 | 0 | 0 | 5 | 5 |
| PH | SOIL-PH-001 – SOIL-PH-007 | 6 | 1 | 0 | 6 | 6 |
| NPK | SOIL-NPK-001 – SOIL-NPK-013 | 13 | 0 | 0 | 13 | 13 |
| NUTR | SOIL-NUTR-001 – SOIL-NUTR-005 | 5 | 0 | 0 | 5 | 5 |
| UI | SOIL-UI-001 – SOIL-UI-018 | 18 | 0 | 0 | 18 | 18 |
| REJOIN | SOIL-REJOIN-001 – SOIL-REJOIN-007 | 0 | 7 | 0 | 0 | 0 |
| **Total** | | **64** | **8** | **0** | **64** | **64** |

**Summary:** 64 of 64 active specs implemented; 8 deferred; 64 implemented specs cited by a test, 0 cited nowhere.

## Key Findings

1. **Deferred:** SOIL-PH-007, SOIL-REJOIN-001, SOIL-REJOIN-002, SOIL-REJOIN-003, SOIL-REJOIN-004, SOIL-REJOIN-005, SOIL-REJOIN-006, SOIL-REJOIN-007.

## Work Required

### Must Fix
- None.

### Should Fix
- None.

### Nice to Have
- None.
