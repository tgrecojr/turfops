# Arrow: seasonal-plan

Seasonal Plan & Calendar — leaf segment `PLAN` under the root HLD.

## Status

**PARTIAL** — audited 2026-10-11 (git SHA `65245e3`): every implemented spec was checked against the code while linking it. 71 of 72 implemented specs are linked to both code and a test; 1 disagree with the code (see Key Findings).

## References

### HLD
- [docs/high-level-design.md](../../docs/high-level-design.md) § System Design — component "Seasonal plan and calendar"

### LLD
- [docs/intent/seasonal-plan/seasonal-plan-design.md](../../docs/intent/seasonal-plan/seasonal-plan-design.md)

### EARS
- [docs/intent/seasonal-plan/seasonal-plan-specs.md](../../docs/intent/seasonal-plan/seasonal-plan-specs.md) (72 specs, prefix `PLAN`)

### Related docs
- [docs/timing-windows.md](../../docs/timing-windows.md)

### Tests
- [backend/src/api/calendar/db_tests.rs](../../backend/src/api/calendar/db_tests.rs)
- [backend/src/api/seasonal_plan/cache_db_tests.rs](../../backend/src/api/seasonal_plan/cache_db_tests.rs)
- [backend/src/api/seasonal_plan/fill_db_tests.rs](../../backend/src/api/seasonal_plan/fill_db_tests.rs)
- [backend/src/api/seasonal_plan/merge_db_tests.rs](../../backend/src/api/seasonal_plan/merge_db_tests.rs)
- [backend/src/logic/seasonal_plan/activity_tests.rs](../../backend/src/logic/seasonal_plan/activity_tests.rs)
- [backend/src/logic/seasonal_plan/completion_tests.rs](../../backend/src/logic/seasonal_plan/completion_tests.rs)
- [backend/src/logic/seasonal_plan/crossing_tests.rs](../../backend/src/logic/seasonal_plan/crossing_tests.rs)
- [backend/src/logic/seasonal_plan/details_tests.rs](../../backend/src/logic/seasonal_plan/details_tests.rs)
- [backend/src/logic/seasonal_plan/status_tests.rs](../../backend/src/logic/seasonal_plan/status_tests.rs)
- [backend/src/logic/seasonal_plan/window_tests.rs](../../backend/src/logic/seasonal_plan/window_tests.rs)
- [frontend/src/components/plan/activityStatus.test.ts](../../frontend/src/components/plan/activityStatus.test.ts)
- [frontend/src/pages/Calendar.test.tsx](../../frontend/src/pages/Calendar.test.tsx)
- [frontend/src/pages/SeasonalPlan.test.tsx](../../frontend/src/pages/SeasonalPlan.test.tsx)

### Code
- [backend/src/api/calendar.rs](../../backend/src/api/calendar.rs)
- [backend/src/api/seasonal_plan.rs](../../backend/src/api/seasonal_plan.rs)
- [backend/src/api/timing.rs](../../backend/src/api/timing.rs)
- [backend/src/clock.rs](../../backend/src/clock.rs)
- [backend/src/datasources/weather.rs](../../backend/src/datasources/weather.rs)
- [backend/src/db/migrations/20240101000006_seasonal_plan_cache.sql](../../backend/src/db/migrations/20240101000006_seasonal_plan_cache.sql)
- [backend/src/db/migrations/20260921000300_threshold_crossings_station.sql](../../backend/src/db/migrations/20260921000300_threshold_crossings_station.sql)
- [backend/src/db/migrations/20261011000000_seasonal_crossing_years.sql](../../backend/src/db/migrations/20261011000000_seasonal_crossing_years.sql)
- [backend/src/db/queries.rs](../../backend/src/db/queries.rs)
- [backend/src/logic/plant_maintenance.rs](../../backend/src/logic/plant_maintenance.rs)
- [backend/src/logic/rules/plan_state.rs](../../backend/src/logic/rules/plan_state.rs)
- [backend/src/logic/seasonal_plan.rs](../../backend/src/logic/seasonal_plan.rs)
- [backend/src/logic/seasonal_plan/crossings.rs](../../backend/src/logic/seasonal_plan/crossings.rs)
- [backend/src/logic/seasonal_plan/fall.rs](../../backend/src/logic/seasonal_plan/fall.rs)
- [backend/src/logic/seasonal_plan/spring.rs](../../backend/src/logic/seasonal_plan/spring.rs)
- [backend/src/logic/timing/plan.rs](../../backend/src/logic/timing/plan.rs)
- [backend/src/mcp/tools/season.rs](../../backend/src/mcp/tools/season.rs)
- [backend/src/models/seasonal_plan.rs](../../backend/src/models/seasonal_plan.rs)
- [frontend/src/components/plan/activityStatus.ts](../../frontend/src/components/plan/activityStatus.ts)
- [frontend/src/pages/Calendar.tsx](../../frontend/src/pages/Calendar.tsx)
- [frontend/src/pages/SeasonalPlan.tsx](../../frontend/src/pages/SeasonalPlan.tsx)
- [frontend/src/types/index.ts](../../frontend/src/types/index.ts)

## Architecture

**Purpose:** see the LLD's Context and Design Philosophy; this page carries pointers and coverage only.

**Spec facets:**
1. `PLAN-CROSS` — 10 cm crossing detection and aggregation
2. `PLAN-CACHE` — crossing cache and its fill
3. `PLAN-ACT` — the plan's own 10 cm activities: windows, completion, status, details
4. `PLAN-MERGE` — `GET /api/v1/seasonal-plan` assembly
5. `PLAN-CAL` — `GET /api/v1/applications/calendar`
6. `PLAN-UI` — Seasonal Plan page
7. `PLAN-CALUI` — Calendar page

## Spec Coverage

Derived from the spec file and a `@spec` scan; regenerate, don't hand-edit.

| Facet | Spec IDs | Implemented | Deferred | Gaps | `@spec` in code | `@spec` in tests |
|---|---|---|---|---|---|---|
| CROSS | PLAN-CROSS-001 – PLAN-CROSS-010 | 10 | 0 | 0 | 10 | 10 |
| CACHE | PLAN-CACHE-001 – PLAN-CACHE-008 | 8 | 0 | 0 | 8 | 8 |
| ACT | PLAN-ACT-001 – PLAN-ACT-023 | 23 | 0 | 0 | 23 | 23 |
| MERGE | PLAN-MERGE-001 – PLAN-MERGE-008 | 8 | 0 | 0 | 7 | 7 |
| CAL | PLAN-CAL-001 – PLAN-CAL-003 | 3 | 0 | 0 | 3 | 3 |
| UI | PLAN-UI-001 – PLAN-UI-011 | 11 | 0 | 0 | 11 | 11 |
| CALUI | PLAN-CALUI-001 – PLAN-CALUI-009 | 9 | 0 | 0 | 9 | 9 |
| **Total** | | **72** | **0** | **0** | **71** | **71** |

**Summary:** 72 of 72 active specs implemented; 0 deferred; 71 implemented specs cited by a test, 1 cited nowhere.

## Key Findings

1. **Spec ≠ code — PLAN-MERGE-006:** contradicts PLANT-PLAN-005: spec says plant activities get applications Oct 1 (prior year) → end of plan year; code passes the log through the end of the latest plant window, which can cross into next year (`logic/plant_maintenance.rs:293` `plant_log_range`), as PLANT-PLAN-005 requires.

## Work Required

### Must Fix
- Decide PLAN-MERGE-006: fix the code or correct the spec, then link it.

### Should Fix
- None.

### Nice to Have
- None.
