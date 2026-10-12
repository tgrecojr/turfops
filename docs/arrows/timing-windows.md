# Arrow: timing-windows

Seeding & Pre-Emergent Timing Windows — leaf segment `TIME` under the root HLD.

## Status

**OK** — audited 2026-10-12 (git SHA `2124c26`): every implemented spec was checked against the code while linking it. 74 of 74 implemented specs are linked to both code and a test.

## References

### HLD
- [docs/high-level-design.md](../../docs/high-level-design.md) § System Design — component "Timing windows"

### LLD
- [docs/intent/timing-windows/timing-windows-design.md](../../docs/intent/timing-windows/timing-windows-design.md)

### EARS
- [docs/intent/timing-windows/timing-windows-specs.md](../../docs/intent/timing-windows/timing-windows-specs.md) (74 specs, prefix `TIME`)

### Related docs
- [docs/agronomy-methods.md](../../docs/agronomy-methods.md)
- [docs/plans/open-meteo-forecast.md](../../docs/plans/open-meteo-forecast.md)
- [docs/timing-windows.md](../../docs/timing-windows.md)

### Tests
- [backend/src/api/timing/handler_tests.rs](../../backend/src/api/timing/handler_tests.rs)
- [backend/src/api/timing/hook_tests.rs](../../backend/src/api/timing/hook_tests.rs)
- [backend/src/api/timing/memo_tests.rs](../../backend/src/api/timing/memo_tests.rs)
- [backend/src/logic/timing/plan/spec_tests.rs](../../backend/src/logic/timing/plan/spec_tests.rs)
- [backend/src/logic/timing/plan/tests.rs](../../backend/src/logic/timing/plan/tests.rs)
- [backend/src/logic/timing/tests.rs](../../backend/src/logic/timing/tests.rs)
- [backend/src/logic/timing/tests/confirmation.rs](../../backend/src/logic/timing/tests/confirmation.rs)
- [backend/src/logic/timing/tests/definitions.rs](../../backend/src/logic/timing/tests/definitions.rs)
- [backend/src/logic/timing/tests/feed.rs](../../backend/src/logic/timing/tests/feed.rs)
- [backend/src/logic/timing/tests/history.rs](../../backend/src/logic/timing/tests/history.rs)
- [backend/src/logic/timing/tests/log_spans.rs](../../backend/src/logic/timing/tests/log_spans.rs)
- [backend/src/logic/timing/tests/outlook.rs](../../backend/src/logic/timing/tests/outlook.rs)
- [backend/src/logic/timing/tests/resolve.rs](../../backend/src/logic/timing/tests/resolve.rs)
- [backend/src/logic/timing/tests/season_context.rs](../../backend/src/logic/timing/tests/season_context.rs)
- [backend/src/logic/timing/tests/states.rs](../../backend/src/logic/timing/tests/states.rs)
- [frontend/src/components/timing/SoilSeasonChart.test.tsx](../../frontend/src/components/timing/SoilSeasonChart.test.tsx)
- [frontend/src/components/timing/StateBadge.test.tsx](../../frontend/src/components/timing/StateBadge.test.tsx)
- [frontend/src/components/timing/TimingWidget.test.tsx](../../frontend/src/components/timing/TimingWidget.test.tsx)
- [frontend/src/components/timing/WindowCard.test.tsx](../../frontend/src/components/timing/WindowCard.test.tsx)
- [frontend/src/components/timing/WindowTimeline.test.tsx](../../frontend/src/components/timing/WindowTimeline.test.tsx)
- [frontend/src/pages/Timing.test.tsx](../../frontend/src/pages/Timing.test.tsx)
- Inline `#[cfg(test)]` modules citing this prefix: `backend/src/api/timing.rs`, `backend/src/logic/timing/climatology.rs`, `backend/src/logic/timing/companions.rs`, `backend/src/logic/timing/context.rs`, `backend/src/logic/timing/evaluate.rs`, `backend/src/logic/timing/log.rs`, `backend/src/logic/timing/recommendations.rs`, `backend/src/logic/timing/series.rs`, `backend/src/logic/timing/text.rs`, `backend/src/logic/timing/windows.rs`

### Code
- [backend/src/api/seasonal_plan.rs](../../backend/src/api/seasonal_plan.rs)
- [backend/src/api/timing.rs](../../backend/src/api/timing.rs)
- [backend/src/api/timing/memo.rs](../../backend/src/api/timing/memo.rs)
- [backend/src/datasources/weather/climatology.rs](../../backend/src/datasources/weather/climatology.rs)
- [backend/src/logic/data_sync.rs](../../backend/src/logic/data_sync.rs)
- [backend/src/logic/timing](../../backend/src/logic/timing)
- [backend/src/logic/timing/climatology.rs](../../backend/src/logic/timing/climatology.rs)
- [backend/src/logic/timing/companions.rs](../../backend/src/logic/timing/companions.rs)
- [backend/src/logic/timing/context.rs](../../backend/src/logic/timing/context.rs)
- [backend/src/logic/timing/evaluate.rs](../../backend/src/logic/timing/evaluate.rs)
- [backend/src/logic/timing/log.rs](../../backend/src/logic/timing/log.rs)
- [backend/src/logic/timing/mod.rs](../../backend/src/logic/timing/mod.rs)
- [backend/src/logic/timing/plan.rs](../../backend/src/logic/timing/plan.rs)
- [backend/src/logic/timing/recommendations.rs](../../backend/src/logic/timing/recommendations.rs)
- [backend/src/logic/timing/series.rs](../../backend/src/logic/timing/series.rs)
- [backend/src/logic/timing/text.rs](../../backend/src/logic/timing/text.rs)
- [backend/src/logic/timing/windows.rs](../../backend/src/logic/timing/windows.rs)
- [backend/src/models/timing.rs](../../backend/src/models/timing.rs)
- [frontend/src/components/timing](../../frontend/src/components/timing)
- [frontend/src/components/timing/SoilSeasonChart.tsx](../../frontend/src/components/timing/SoilSeasonChart.tsx)
- [frontend/src/components/timing/StateBadge.tsx](../../frontend/src/components/timing/StateBadge.tsx)
- [frontend/src/components/timing/TimingWidget.tsx](../../frontend/src/components/timing/TimingWidget.tsx)
- [frontend/src/components/timing/WindowCard.tsx](../../frontend/src/components/timing/WindowCard.tsx)
- [frontend/src/components/timing/WindowTimeline.tsx](../../frontend/src/components/timing/WindowTimeline.tsx)
- [frontend/src/pages/Timing.tsx](../../frontend/src/pages/Timing.tsx)
- [frontend/src/types/timing.ts](../../frontend/src/types/timing.ts)

## Architecture

**Purpose:** see the LLD's Context and Design Philosophy; this page carries pointers and coverage only.

## Spec Coverage

Derived from the spec file and a `@spec` scan; regenerate, don't hand-edit.

| Facet | Spec IDs | Implemented | Deferred | Gaps | `@spec` in code | `@spec` in tests |
|---|---|---|---|---|---|---|
| ENG | TIME-ENG-001 – TIME-ENG-046 | 46 | 0 | 0 | 46 | 46 |
| FEED | TIME-FEED-001 – TIME-FEED-009 | 9 | 0 | 0 | 9 | 9 |
| LOG | TIME-LOG-001 – TIME-LOG-004 | 4 | 0 | 0 | 4 | 4 |
| PLAN | TIME-PLAN-001 – TIME-PLAN-007 | 7 | 0 | 0 | 7 | 7 |
| UI | TIME-UI-001 – TIME-UI-008 | 8 | 0 | 0 | 8 | 8 |
| **Total** | | **74** | **0** | **0** | **74** | **74** |

**Summary:** 74 of 74 active specs implemented; 0 deferred; 74 implemented specs cited by a test, 0 cited nowhere.

## Key Findings

1. No findings.

## Work Required

### Must Fix
- None.

### Should Fix
- None.

### Nice to Have
- None.
