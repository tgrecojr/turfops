# Arrow: landscape-plants

Landscape Plants — leaf segment `PLANT` under the root HLD.

## Status

**OK** — audited 2026-10-12 (git SHA `2bf061e`): every implemented spec was checked against the code while linking it. 79 of 79 implemented specs are linked to both code and a test.

## References

### HLD
- [docs/high-level-design.md](../../docs/high-level-design.md) § System Design — component "Landscape plants"

### LLD
- [docs/intent/landscape-plants/landscape-plants-design.md](../../docs/intent/landscape-plants/landscape-plants-design.md)

### EARS
- [docs/intent/landscape-plants/landscape-plants-specs.md](../../docs/intent/landscape-plants/landscape-plants-specs.md) (79 specs, prefix `PLANT`)

### Related docs
- [docs/agronomy-methods.md](../../docs/agronomy-methods.md)

### Tests
- [backend/src/api/plants/db_crud_tests.rs](../../backend/src/api/plants/db_crud_tests.rs)
- [backend/src/api/plants/db_tests.rs](../../backend/src/api/plants/db_tests.rs)
- [backend/src/api/plants/tests.rs](../../backend/src/api/plants/tests.rs)
- [backend/src/datasources/openrouter/build_tests.rs](../../backend/src/datasources/openrouter/build_tests.rs)
- [backend/src/datasources/openrouter/client_tests.rs](../../backend/src/datasources/openrouter/client_tests.rs)
- [backend/src/datasources/openrouter/prompt_spec_tests.rs](../../backend/src/datasources/openrouter/prompt_spec_tests.rs)
- [backend/src/db/plant_queries/row_tests.rs](../../backend/src/db/plant_queries/row_tests.rs)
- [backend/src/logic/plant_maintenance/feed_spec_tests.rs](../../backend/src/logic/plant_maintenance/feed_spec_tests.rs)
- [backend/src/logic/plant_maintenance/spec_tests.rs](../../backend/src/logic/plant_maintenance/spec_tests.rs)
- [backend/src/logic/plant_maintenance/tests.rs](../../backend/src/logic/plant_maintenance/tests.rs)
- [backend/src/logic/plant_maintenance/water_spec_tests.rs](../../backend/src/logic/plant_maintenance/water_spec_tests.rs)
- [backend/src/logic/plant_maintenance/window_tests.rs](../../backend/src/logic/plant_maintenance/window_tests.rs)
- [frontend/src/api/plantClient.test.ts](../../frontend/src/api/plantClient.test.ts)
- [frontend/src/components/landscape/plantEdit.test.ts](../../frontend/src/components/landscape/plantEdit.test.ts)
- [frontend/src/pages/Landscape.test.tsx](../../frontend/src/pages/Landscape.test.tsx)
- [frontend/src/pages/LandscapeAdd.test.tsx](../../frontend/src/pages/LandscapeAdd.test.tsx)
- [frontend/src/pages/LandscapeRoute.test.tsx](../../frontend/src/pages/LandscapeRoute.test.tsx)
- Inline `#[cfg(test)]` modules citing this prefix: `backend/src/config.rs`, `backend/src/datasources/openrouter.rs`, `backend/src/datasources/openrouter/plant.rs`, `backend/src/db/plant_queries.rs`, `backend/src/error.rs`, `backend/src/logic/plant_maintenance/watering.rs`, `backend/src/models/plant.rs`

### Code
- [backend/src/api/plants.rs](../../backend/src/api/plants.rs)
- [backend/src/api/recommendation_feed.rs](../../backend/src/api/recommendation_feed.rs)
- [backend/src/api/seasonal_plan.rs](../../backend/src/api/seasonal_plan.rs)
- [backend/src/config.rs](../../backend/src/config.rs)
- [backend/src/datasources/openrouter.rs](../../backend/src/datasources/openrouter.rs)
- [backend/src/datasources/openrouter/plant.rs](../../backend/src/datasources/openrouter/plant.rs)
- [backend/src/datasources/openrouter/product.rs](../../backend/src/datasources/openrouter/product.rs)
- [backend/src/db/migrations/20260425000000_plants.sql](../../backend/src/db/migrations/20260425000000_plants.sql)
- [backend/src/db/plant_queries.rs](../../backend/src/db/plant_queries.rs)
- [backend/src/error.rs](../../backend/src/error.rs)
- [backend/src/logic/plant_maintenance.rs](../../backend/src/logic/plant_maintenance.rs)
- [backend/src/logic/plant_maintenance/watering.rs](../../backend/src/logic/plant_maintenance/watering.rs)
- [backend/src/logic/rules/thresholds.rs](../../backend/src/logic/rules/thresholds.rs)
- [backend/src/main.rs](../../backend/src/main.rs)
- [backend/src/mcp/tools/shelf.rs](../../backend/src/mcp/tools/shelf.rs)
- [backend/src/models/plant.rs](../../backend/src/models/plant.rs)
- [frontend/src/App.tsx](../../frontend/src/App.tsx)
- [frontend/src/api/client.ts](../../frontend/src/api/client.ts)
- [frontend/src/components/Layout.tsx](../../frontend/src/components/Layout.tsx)
- [frontend/src/components/landscape/EditPlantForm.tsx](../../frontend/src/components/landscape/EditPlantForm.tsx)
- [frontend/src/components/landscape/plantEdit.ts](../../frontend/src/components/landscape/plantEdit.ts)
- [frontend/src/pages/Landscape.tsx](../../frontend/src/pages/Landscape.tsx)

## Architecture

**Purpose:** see the LLD's Context and Design Philosophy; this page carries pointers and coverage only.

**Spec facets:**
1. `PLANT-DATA` — plant model and stored plan
2. `PLANT-API` — plants endpoints
3. `PLANT-AI` — shared OpenRouter client
4. `PLANT-PROMPT` — plant care-plan prompt and schema
5. `PLANT-WIN` — maintenance windows and completion
6. `PLANT-REC` — plant task recommendations
7. `PLANT-WATER` — watering hold-back
8. `PLANT-PLAN` — seasonal-plan activities
9. `PLANT-UI` — Landscape page

## Spec Coverage

Derived from the spec file and a `@spec` scan; regenerate, don't hand-edit.

| Facet | Spec IDs | Implemented | Deferred | Gaps | `@spec` in code | `@spec` in tests |
|---|---|---|---|---|---|---|
| DATA | PLANT-DATA-001 – PLANT-DATA-007 | 7 | 0 | 0 | 7 | 7 |
| API | PLANT-API-001 – PLANT-API-013 | 13 | 0 | 0 | 13 | 13 |
| AI | PLANT-AI-001 – PLANT-AI-010 | 10 | 0 | 0 | 10 | 10 |
| PROMPT | PLANT-PROMPT-001 – PLANT-PROMPT-009 | 9 | 0 | 0 | 9 | 9 |
| WIN | PLANT-WIN-001 – PLANT-WIN-007 | 7 | 0 | 0 | 7 | 7 |
| REC | PLANT-REC-001 – PLANT-REC-008 | 8 | 0 | 0 | 8 | 8 |
| WATER | PLANT-WATER-001 – PLANT-WATER-007 | 7 | 0 | 0 | 7 | 7 |
| PLAN | PLANT-PLAN-001 – PLANT-PLAN-005 | 5 | 0 | 0 | 5 | 5 |
| UI | PLANT-UI-001 – PLANT-UI-013 | 13 | 0 | 0 | 13 | 13 |
| **Total** | | **79** | **0** | **0** | **79** | **79** |

**Summary:** 79 of 79 active specs implemented; 0 deferred; 79 implemented specs cited by a test, 0 cited nowhere.

## Key Findings

1. No findings.

## Work Required

### Must Fix
- None.

### Should Fix
- None.

### Nice to Have
- None.
