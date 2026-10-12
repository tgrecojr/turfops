# Arrow: agronomic-rules

Agronomic Rules — leaf segment `RULE` under the root HLD.

## Status

**OK** — audited 2026-10-12 (git SHA `0e3be7a`): every implemented spec was checked against the code while linking it. 76 of 76 implemented specs are linked to both code and a test.

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
- [backend/src/api/gdd/handler_tests.rs](../../backend/src/api/gdd/handler_tests.rs)
- [backend/src/api/nitrogen_budget/db_tests.rs](../../backend/src/api/nitrogen_budget/db_tests.rs)
- [backend/src/datasources/weather/tests.rs](../../backend/src/datasources/weather/tests.rs)
- [backend/src/logic/gdd/lake_tests.rs](../../backend/src/logic/gdd/lake_tests.rs)
- [backend/src/logic/rules/broadleaf_herbicide/rule_tests.rs](../../backend/src/logic/rules/broadleaf_herbicide/rule_tests.rs)
- [backend/src/logic/rules/engine_tests.rs](../../backend/src/logic/rules/engine_tests.rs)
- [backend/src/logic/rules/fall_fertilization/rule_tests.rs](../../backend/src/logic/rules/fall_fertilization/rule_tests.rs)
- [backend/src/logic/rules/forecast_rule_tests.rs](../../backend/src/logic/rules/forecast_rule_tests.rs)
- [backend/src/logic/rules/grub_control/rule_tests.rs](../../backend/src/logic/rules/grub_control/rule_tests.rs)
- [backend/src/logic/rules/moment_tests.rs](../../backend/src/logic/rules/moment_tests.rs)
- [backend/src/logic/rules/nitrogen_guard/rule_tests.rs](../../backend/src/logic/rules/nitrogen_guard/rule_tests.rs)
- [backend/src/logic/rules/plan_state/tests.rs](../../backend/src/logic/rules/plan_state/tests.rs)
- [backend/src/logic/rules/plan_state/window_tests.rs](../../backend/src/logic/rules/plan_state/window_tests.rs)
- [backend/src/logic/rules/rain_delay_tests.rs](../../backend/src/logic/rules/rain_delay_tests.rs)
- [backend/src/logic/rules/spring_nitrogen/rule_tests.rs](../../backend/src/logic/rules/spring_nitrogen/rule_tests.rs)
- [backend/src/logic/rules/weather_rule_tests.rs](../../backend/src/logic/rules/weather_rule_tests.rs)
- [frontend/src/components/GddWidget.test.tsx](../../frontend/src/components/GddWidget.test.tsx)
- [frontend/src/components/NitrogenBudgetWidget.test.tsx](../../frontend/src/components/NitrogenBudgetWidget.test.tsx)
- Inline `#[cfg(test)]` modules citing this prefix: `backend/src/api/nitrogen_budget.rs`, `backend/src/logic/rules/fall_fertilization/recs.rs`, `backend/src/logic/rules/nitrogen_guard.rs`, `backend/src/logic/rules/spring_nitrogen.rs`, `backend/src/models/nitrogen_budget.rs`

### Code
- [backend/src/api/gdd.rs](../../backend/src/api/gdd.rs)
- [backend/src/api/nitrogen_budget.rs](../../backend/src/api/nitrogen_budget.rs)
- [backend/src/api/recommendation_feed.rs](../../backend/src/api/recommendation_feed.rs)
- [backend/src/clock.rs](../../backend/src/clock.rs)
- [backend/src/datasources/weather.rs](../../backend/src/datasources/weather.rs)
- [backend/src/logic/gdd.rs](../../backend/src/logic/gdd.rs)
- [backend/src/logic/plant_maintenance/watering.rs](../../backend/src/logic/plant_maintenance/watering.rs)
- [backend/src/logic/rules](../../backend/src/logic/rules)
- [backend/src/logic/rules/application_window.rs](../../backend/src/logic/rules/application_window.rs)
- [backend/src/logic/rules/broadleaf_herbicide.rs](../../backend/src/logic/rules/broadleaf_herbicide.rs)
- [backend/src/logic/rules/engine.rs](../../backend/src/logic/rules/engine.rs)
- [backend/src/logic/rules/fall_fertilization.rs](../../backend/src/logic/rules/fall_fertilization.rs)
- [backend/src/logic/rules/fall_fertilization/recs.rs](../../backend/src/logic/rules/fall_fertilization/recs.rs)
- [backend/src/logic/rules/fertilizer.rs](../../backend/src/logic/rules/fertilizer.rs)
- [backend/src/logic/rules/grub_control.rs](../../backend/src/logic/rules/grub_control.rs)
- [backend/src/logic/rules/heat_stress.rs](../../backend/src/logic/rules/heat_stress.rs)
- [backend/src/logic/rules/irrigation_forecast.rs](../../backend/src/logic/rules/irrigation_forecast.rs)
- [backend/src/logic/rules/mod.rs](../../backend/src/logic/rules/mod.rs)
- [backend/src/logic/rules/mowing_height.rs](../../backend/src/logic/rules/mowing_height.rs)
- [backend/src/logic/rules/nitrogen_guard.rs](../../backend/src/logic/rules/nitrogen_guard.rs)
- [backend/src/logic/rules/plan_state.rs](../../backend/src/logic/rules/plan_state.rs)
- [backend/src/logic/rules/rain_delay.rs](../../backend/src/logic/rules/rain_delay.rs)
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
| ENG | RULE-ENG-001 – RULE-ENG-015 | 15 | 0 | 0 | 15 | 15 |
| SPRING | RULE-SPRING-001 – RULE-SPRING-010 | 10 | 0 | 0 | 10 | 10 |
| BROAD | RULE-BROAD-001 – RULE-BROAD-004 | 4 | 0 | 0 | 4 | 4 |
| GRUB | RULE-GRUB-001 – RULE-GRUB-006 | 6 | 0 | 0 | 6 | 6 |
| BLOCK | RULE-BLOCK-001 – RULE-BLOCK-002 | 2 | 0 | 0 | 2 | 2 |
| FALL | RULE-FALL-001 – RULE-FALL-007 | 7 | 0 | 0 | 7 | 7 |
| WX | RULE-WX-001 – RULE-WX-008 | 8 | 0 | 0 | 8 | 8 |
| GUARD | RULE-GUARD-001 – RULE-GUARD-011 | 10 | 1 | 0 | 10 | 10 |
| BUDGET | RULE-BUDGET-001 – RULE-BUDGET-005 | 5 | 0 | 0 | 5 | 5 |
| GDD | RULE-GDD-001 – RULE-GDD-007 | 7 | 0 | 0 | 7 | 7 |
| UI | RULE-UI-001 – RULE-UI-002 | 2 | 0 | 0 | 2 | 2 |
| **Total** | | **76** | **1** | **0** | **76** | **76** |

**Summary:** 76 of 76 active specs implemented; 1 deferred; 76 implemented specs cited by a test, 0 cited nowhere.

## Key Findings

1. **Deferred:** RULE-GUARD-005.

## Work Required

### Must Fix
- None.

### Should Fix
- None.

### Nice to Have
- None.
