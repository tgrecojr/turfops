# Arrow: seasonal-plan

Seasonal Plan & Calendar — leaf segment `PLAN` under the root HLD.

## Status

**MAPPED** — overlay bootstrapped 2026-10-11 from the existing LID docs; not yet audited (`audited_sha: null`). 72 specs `[x]`, 0 `[D]`, 0 `[ ]`; 41 of 72 implemented specs carry a `@spec` annotation.

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
- [backend/src/logic/seasonal_plan/completion_tests.rs](../../backend/src/logic/seasonal_plan/completion_tests.rs)
- [backend/src/logic/seasonal_plan/crossing_tests.rs](../../backend/src/logic/seasonal_plan/crossing_tests.rs)
- [backend/src/logic/seasonal_plan/status_tests.rs](../../backend/src/logic/seasonal_plan/status_tests.rs)
- [backend/src/logic/seasonal_plan/window_tests.rs](../../backend/src/logic/seasonal_plan/window_tests.rs)
- [frontend/src/components/plan/activityStatus.test.ts](../../frontend/src/components/plan/activityStatus.test.ts)

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
| CROSS | PLAN-CROSS-001 – PLAN-CROSS-010 | 10 | 0 | 0 | 9 | 8 |
| CACHE | PLAN-CACHE-001 – PLAN-CACHE-008 | 8 | 0 | 0 | 6 | 2 |
| ACT | PLAN-ACT-001 – PLAN-ACT-023 | 23 | 0 | 0 | 7 | 15 |
| MERGE | PLAN-MERGE-001 – PLAN-MERGE-008 | 8 | 0 | 0 | 0 | 1 |
| CAL | PLAN-CAL-001 – PLAN-CAL-003 | 3 | 0 | 0 | 0 | 0 |
| UI | PLAN-UI-001 – PLAN-UI-011 | 11 | 0 | 0 | 5 | 4 |
| CALUI | PLAN-CALUI-001 – PLAN-CALUI-009 | 9 | 0 | 0 | 5 | 4 |
| **Total** | | **72** | **0** | **0** | **32** | **34** |

**Summary:** 72 of 72 active specs implemented; 0 deferred; 34 implemented specs cited by a test, 31 cited nowhere.

## Key Findings

1. **31 implemented specs have no `@spec` annotation** — the specs were written to describe existing code and never linked back: PLAN-CROSS-007; PLAN-CACHE-001/004; PLAN-ACT-001/002/003/009/013/014/015/016; PLAN-MERGE-001/002/003/004/005/006/007; PLAN-CAL-001/002/003; PLAN-UI-002/003/005/006/008/009; PLAN-CALUI-002/003/004/007.
2. **7 specs are annotated in code but cited by no test** — PLAN-CROSS-002; PLAN-CACHE-002/003/006/007; PLAN-UI-010; PLAN-CALUI-008.

## Work Required

### Must Fix
- None.

### Should Fix
1. Add `@spec` annotations (entry point + exercising test) for the 31 unlinked specs.
2. Annotate a test for the 7 code-only specs.

### Nice to Have
- Audit each `[x]` against the code and move the segment to `AUDITED`.
