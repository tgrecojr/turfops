# Arrow: soil-tests

Soil Tests — leaf segment `SOIL` under the root HLD.

## Status

**MAPPED** — overlay bootstrapped 2026-10-11 from the existing LID docs; not yet audited (`audited_sha: null`). 64 specs `[x]`, 8 `[D]`, 0 `[ ]`; 17 of 64 implemented specs carry a `@spec` annotation.

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
- [backend/src/api/soil_tests.rs](../../backend/src/api/soil_tests.rs)
- [backend/src/api/soil_tests/tests.rs](../../backend/src/api/soil_tests/tests.rs)
- [backend/src/logic/soil_test_recommendations/gap_tests.rs](../../backend/src/logic/soil_test_recommendations/gap_tests.rs)
- [backend/src/logic/soil_test_recommendations/tests.rs](../../backend/src/logic/soil_test_recommendations/tests.rs)
- [backend/src/models/soil_test.rs](../../backend/src/models/soil_test.rs)
- [frontend/src/components/soilTests/display.test.ts](../../frontend/src/components/soilTests/display.test.ts)

### Code
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
| DATA | SOIL-DATA-001 – SOIL-DATA-005 | 5 | 0 | 0 | 1 | 1 |
| API | SOIL-API-001 – SOIL-API-012 | 12 | 0 | 0 | 0 | 2 |
| ADV | SOIL-ADV-001 – SOIL-ADV-005 | 5 | 0 | 0 | 0 | 0 |
| PH | SOIL-PH-001 – SOIL-PH-007 | 6 | 1 | 0 | 0 | 0 |
| NPK | SOIL-NPK-001 – SOIL-NPK-013 | 13 | 0 | 0 | 4 | 4 |
| NUTR | SOIL-NUTR-001 – SOIL-NUTR-005 | 5 | 0 | 0 | 2 | 2 |
| UI | SOIL-UI-001 – SOIL-UI-018 | 18 | 0 | 0 | 8 | 7 |
| REJOIN | SOIL-REJOIN-001 – SOIL-REJOIN-007 | 0 | 7 | 0 | 0 | 0 |
| **Total** | | **64** | **8** | **0** | **15** | **16** |

**Summary:** 64 of 64 active specs implemented; 8 deferred; 16 implemented specs cited by a test, 47 cited nowhere.

## Key Findings

1. **47 implemented specs have no `@spec` annotation** — the specs were written to describe existing code and never linked back: SOIL-DATA-001/002/003/004; SOIL-API-001/002/003/004/005/006/007/008/009/010; SOIL-ADV-001/002/003/004/005; SOIL-PH-001/002/003/004/005/006; SOIL-NPK-001/002/003/005/006/007/008/009/011; SOIL-NUTR-001/002/003; SOIL-UI-001/002/003/004/005/006/007/008/010/011.
2. **1 specs are annotated in code but cited by no test** — SOIL-UI-013.
3. **Deferred:** SOIL-PH-007, SOIL-REJOIN-001, SOIL-REJOIN-002, SOIL-REJOIN-003, SOIL-REJOIN-004, SOIL-REJOIN-005, SOIL-REJOIN-006, SOIL-REJOIN-007.

## Work Required

### Must Fix
- None.

### Should Fix
1. Add `@spec` annotations (entry point + exercising test) for the 47 unlinked specs.
2. Annotate a test for the 1 code-only specs.

### Nice to Have
- Audit each `[x]` against the code and move the segment to `AUDITED`.
