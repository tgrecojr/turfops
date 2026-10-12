# Arrow: landscape-plants

Landscape Plants — leaf segment `PLANT` under the root HLD.

## Status

**MAPPED** — overlay bootstrapped 2026-10-11 from the existing LID docs; not yet audited (`audited_sha: null`). 79 specs `[x]`, 0 `[D]`, 0 `[ ]`; 18 of 79 implemented specs carry a `@spec` annotation.

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
- [backend/src/api/plants/tests.rs](../../backend/src/api/plants/tests.rs)
- [backend/src/logic/plant_maintenance/tests.rs](../../backend/src/logic/plant_maintenance/tests.rs)
- [backend/src/logic/plant_maintenance/window_tests.rs](../../backend/src/logic/plant_maintenance/window_tests.rs)
- [frontend/src/components/landscape/plantEdit.test.ts](../../frontend/src/components/landscape/plantEdit.test.ts)
- Inline `#[cfg(test)]` modules citing this prefix: `backend/src/config.rs`, `backend/src/datasources/openrouter.rs`, `backend/src/datasources/openrouter/plant.rs`, `backend/src/db/plant_queries.rs`, `backend/src/error.rs`

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
- [frontend/src/api/client.ts](../../frontend/src/api/client.ts)
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
| DATA | PLANT-DATA-001 – PLANT-DATA-007 | 7 | 0 | 0 | 1 | 1 |
| API | PLANT-API-001 – PLANT-API-013 | 13 | 0 | 0 | 2 | 2 |
| AI | PLANT-AI-001 – PLANT-AI-010 | 10 | 0 | 0 | 2 | 3 |
| PROMPT | PLANT-PROMPT-001 – PLANT-PROMPT-009 | 9 | 0 | 0 | 2 | 2 |
| WIN | PLANT-WIN-001 – PLANT-WIN-007 | 7 | 0 | 0 | 1 | 1 |
| REC | PLANT-REC-001 – PLANT-REC-008 | 8 | 0 | 0 | 3 | 3 |
| WATER | PLANT-WATER-001 – PLANT-WATER-007 | 7 | 0 | 0 | 0 | 0 |
| PLAN | PLANT-PLAN-001 – PLANT-PLAN-005 | 5 | 0 | 0 | 2 | 2 |
| UI | PLANT-UI-001 – PLANT-UI-013 | 13 | 0 | 0 | 4 | 2 |
| **Total** | | **79** | **0** | **0** | **17** | **16** |

**Summary:** 79 of 79 active specs implemented; 0 deferred; 16 implemented specs cited by a test, 61 cited nowhere.

## Key Findings

1. **61 implemented specs have no `@spec` annotation** — the specs were written to describe existing code and never linked back: PLANT-DATA-001/002/003/004/005/007; PLANT-API-001/002/003/005/006/007/008/009/010/011/012; PLANT-AI-001/002/003/004/005/006/009; PLANT-PROMPT-001/002/003/004/005/008/009; PLANT-WIN-001/002/003/004/005/007; PLANT-REC-001/003/004/006/007; PLANT-WATER-001/002/003/004/005/006/007; PLANT-PLAN-001/002/004; PLANT-UI-001/002/003/004/005/006/007/008/009.
2. **2 specs are annotated in code but cited by no test** — PLANT-UI-010/011.

## Work Required

### Must Fix
- None.

### Should Fix
1. Add `@spec` annotations (entry point + exercising test) for the 61 unlinked specs.
2. Annotate a test for the 2 code-only specs.

### Nice to Have
- Audit each `[x]` against the code and move the segment to `AUDITED`.
