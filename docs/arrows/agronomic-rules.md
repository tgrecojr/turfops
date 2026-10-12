# Arrow: agronomic-rules

Agronomic Rules — leaf segment `RULE` under the root HLD.

## Status

**MAPPED** — overlay bootstrapped 2026-10-11 from the existing LID docs; not yet audited (`audited_sha: null`). 76 specs `[x]`, 1 `[D]`, 0 `[ ]`; 21 of 76 implemented specs carry a `@spec` annotation.

## References

### HLD
- [docs/high-level-design.md](../../docs/high-level-design.md) § System Design — component "Agronomic rules"

### LLD
- [docs/intent/agronomic-rules/agronomic-rules-design.md](../../docs/intent/agronomic-rules/agronomic-rules-design.md)

### EARS
- [docs/intent/agronomic-rules/agronomic-rules-specs.md](../../docs/intent/agronomic-rules/agronomic-rules-specs.md) (77 specs, prefix `RULE`)

### Related docs
- [docs/agronomy-methods.md](../../docs/agronomy-methods.md)

### Tests
- [backend/src/datasources/weather/tests.rs](../../backend/src/datasources/weather/tests.rs)
- [backend/src/logic/rules/plan_state/tests.rs](../../backend/src/logic/rules/plan_state/tests.rs)
- [backend/src/logic/rules/plan_state/window_tests.rs](../../backend/src/logic/rules/plan_state/window_tests.rs)
- Inline `#[cfg(test)]` modules citing this prefix: `backend/src/api/nitrogen_budget.rs`, `backend/src/logic/rules/fall_fertilization/recs.rs`, `backend/src/logic/rules/nitrogen_guard.rs`, `backend/src/logic/rules/spring_nitrogen.rs`

### Code
- [backend/src/api/gdd.rs](../../backend/src/api/gdd.rs)
- [backend/src/api/nitrogen_budget.rs](../../backend/src/api/nitrogen_budget.rs)
- [backend/src/api/recommendation_feed.rs](../../backend/src/api/recommendation_feed.rs)
- [backend/src/clock.rs](../../backend/src/clock.rs)
- [backend/src/datasources/weather.rs](../../backend/src/datasources/weather.rs)
- [backend/src/logic/gdd.rs](../../backend/src/logic/gdd.rs)
- [backend/src/logic/plant_maintenance/watering.rs](../../backend/src/logic/plant_maintenance/watering.rs)
- [backend/src/logic/rules](../../backend/src/logic/rules)
- [backend/src/logic/rules/fall_fertilization/recs.rs](../../backend/src/logic/rules/fall_fertilization/recs.rs)
- [backend/src/logic/rules/nitrogen_guard.rs](../../backend/src/logic/rules/nitrogen_guard.rs)
- [backend/src/logic/rules/plan_state.rs](../../backend/src/logic/rules/plan_state.rs)
- [backend/src/logic/rules/spring_nitrogen.rs](../../backend/src/logic/rules/spring_nitrogen.rs)
- [backend/src/logic/timing/companions.rs](../../backend/src/logic/timing/companions.rs)
- [backend/src/logic/timing/recommendations.rs](../../backend/src/logic/timing/recommendations.rs)
- [backend/src/models/application.rs](../../backend/src/models/application.rs)
- [backend/src/models/gdd.rs](../../backend/src/models/gdd.rs)
- [backend/src/models/inventory.rs](../../backend/src/models/inventory.rs)
- [backend/src/models/lawn_profile.rs](../../backend/src/models/lawn_profile.rs)
- [backend/src/models/nitrogen_budget.rs](../../backend/src/models/nitrogen_budget.rs)
- [backend/src/models/recommendation.rs](../../backend/src/models/recommendation.rs)
- [backend/src/state.rs](../../backend/src/state.rs)
- [frontend/src/components/GddWidget.tsx](../../frontend/src/components/GddWidget.tsx)
- [frontend/src/components/NitrogenBudgetWidget.tsx](../../frontend/src/components/NitrogenBudgetWidget.tsx)
- [frontend/src/pages/Dashboard.tsx](../../frontend/src/pages/Dashboard.tsx)
- [frontend/src/types/index.ts](../../frontend/src/types/index.ts)

## Architecture

**Purpose:** see the LLD's Context and Design Philosophy; this page carries pointers and coverage only.

**Spec facets:**
1. `RULE-ENG` — rule contract and engine
2. `RULE-SPRING` — spring nitrogen
3. `RULE-BROAD` — broadleaf herbicide
4. `RULE-GRUB` — grub control
5. `RULE-BLOCK` — fertilizer block
6. `RULE-FALL` — fall fertilization
7. `RULE-WX` — rain delay, irrigation, heat stress, application window, mowing height
8. `RULE-GUARD` — nitrogen guard
9. `RULE-BUDGET` — annual nitrogen budget
10. `RULE-GDD` — growing degree days and the crabgrass model
11. `RULE-UI` — dashboard widgets

## Spec Coverage

Derived from the spec file and a `@spec` scan; regenerate, don't hand-edit.

| Facet | Spec IDs | Implemented | Deferred | Gaps | `@spec` in code | `@spec` in tests |
|---|---|---|---|---|---|---|
| ENG | RULE-ENG-001 – RULE-ENG-015 | 15 | 0 | 0 | 7 | 7 |
| SPRING | RULE-SPRING-001 – RULE-SPRING-010 | 10 | 0 | 0 | 2 | 2 |
| BROAD | RULE-BROAD-001 – RULE-BROAD-004 | 4 | 0 | 0 | 0 | 0 |
| GRUB | RULE-GRUB-001 – RULE-GRUB-006 | 6 | 0 | 0 | 0 | 0 |
| BLOCK | RULE-BLOCK-001 – RULE-BLOCK-002 | 2 | 0 | 0 | 0 | 0 |
| FALL | RULE-FALL-001 – RULE-FALL-007 | 7 | 0 | 0 | 1 | 1 |
| WX | RULE-WX-001 – RULE-WX-008 | 8 | 0 | 0 | 0 | 0 |
| GUARD | RULE-GUARD-001 – RULE-GUARD-011 | 10 | 1 | 0 | 8 | 4 |
| BUDGET | RULE-BUDGET-001 – RULE-BUDGET-005 | 5 | 0 | 0 | 1 | 1 |
| GDD | RULE-GDD-001 – RULE-GDD-007 | 7 | 0 | 0 | 2 | 1 |
| UI | RULE-UI-001 – RULE-UI-002 | 2 | 0 | 0 | 0 | 0 |
| **Total** | | **76** | **1** | **0** | **21** | **16** |

**Summary:** 76 of 76 active specs implemented; 1 deferred; 16 implemented specs cited by a test, 55 cited nowhere.

## Key Findings

1. **55 implemented specs have no `@spec` annotation** — the specs were written to describe existing code and never linked back: RULE-ENG-001/002/003/004/005/006/007/008; RULE-SPRING-001/002/003/004/005/006/007/009; RULE-BROAD-001/002/003/004; RULE-GRUB-001/002/003/004/005/006; RULE-BLOCK-001/002; RULE-FALL-001/002/003/004/005/006; RULE-WX-001/002/003/004/005/006/007/008; RULE-GUARD-009/010; RULE-BUDGET-001/002/003/004; RULE-GDD-001/003/004/005/006; RULE-UI-001/002.
2. **5 specs are annotated in code but cited by no test** — RULE-GUARD-003/004/006/008; RULE-GDD-007.
3. **Deferred:** RULE-GUARD-005.

## Work Required

### Must Fix
- None.

### Should Fix
1. Add `@spec` annotations (entry point + exercising test) for the 55 unlinked specs.
2. Annotate a test for the 5 code-only specs.

### Nice to Have
- Audit each `[x]` against the code and move the segment to `AUDITED`.
