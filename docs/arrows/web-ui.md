# Arrow: web-ui

Web UI Shell & Conventions — leaf segment `UI` under the root HLD.

## Status

**MAPPED** — overlay bootstrapped 2026-10-11 from the existing LID docs; not yet audited (`audited_sha: null`). 63 specs `[x]`, 0 `[D]`, 0 `[ ]`; 23 of 63 implemented specs carry a `@spec` annotation.

## References

### HLD
- [docs/high-level-design.md](../../docs/high-level-design.md) § System Design — component "Web UI"

### LLD
- [docs/intent/web-ui/web-ui-design.md](../../docs/intent/web-ui/web-ui-design.md)

### EARS
- [docs/intent/web-ui/web-ui-specs.md](../../docs/intent/web-ui/web-ui-specs.md) (63 specs, prefix `UI`)

### Related docs
- [docs/timing-windows.md](../../docs/timing-windows.md)

### Tests
- [backend/src/config/load_tests.rs](../../backend/src/config/load_tests.rs)
- [backend/src/web/tests.rs](../../backend/src/web/tests.rs)
- [frontend/src/api/client.test.ts](../../frontend/src/api/client.test.ts)
- [frontend/src/api/errors.test.ts](../../frontend/src/api/errors.test.ts)
- [frontend/src/components/dashboard/loadDashboard.test.ts](../../frontend/src/components/dashboard/loadDashboard.test.ts)
- [frontend/src/utils/dates.test.ts](../../frontend/src/utils/dates.test.ts)
- [frontend/tests/repo.test.ts](../../frontend/tests/repo.test.ts)
- Inline `#[cfg(test)]` modules citing this prefix: `backend/src/api/json_errors.rs`, `backend/src/error.rs`

### Code
- [.github/workflows/ci.yml](../../.github/workflows/ci.yml)
- [.github/workflows/docker-publish.yml](../../.github/workflows/docker-publish.yml)
- [backend/src/api/json_errors.rs](../../backend/src/api/json_errors.rs)
- [backend/src/config.rs](../../backend/src/config.rs)
- [backend/src/config/load.rs](../../backend/src/config/load.rs)
- [backend/src/error.rs](../../backend/src/error.rs)
- [backend/src/main.rs](../../backend/src/main.rs)
- [backend/src/web.rs](../../backend/src/web.rs)
- [frontend/README.md](../../frontend/README.md)
- [frontend/biome.json](../../frontend/biome.json)
- [frontend/index.html](../../frontend/index.html)
- [frontend/package.json](../../frontend/package.json)
- [frontend/src/App.tsx](../../frontend/src/App.tsx)
- [frontend/src/api/client.ts](../../frontend/src/api/client.ts)
- [frontend/src/api/errors.ts](../../frontend/src/api/errors.ts)
- [frontend/src/components/AlertCard.tsx](../../frontend/src/components/AlertCard.tsx)
- [frontend/src/components/ChartTable.tsx](../../frontend/src/components/ChartTable.tsx)
- [frontend/src/components/Gauge.tsx](../../frontend/src/components/Gauge.tsx)
- [frontend/src/components/GddWidget.tsx](../../frontend/src/components/GddWidget.tsx)
- [frontend/src/components/InventoryBadge.tsx](../../frontend/src/components/InventoryBadge.tsx)
- [frontend/src/components/Layout.tsx](../../frontend/src/components/Layout.tsx)
- [frontend/src/components/NitrogenBudgetWidget.tsx](../../frontend/src/components/NitrogenBudgetWidget.tsx)
- [frontend/src/components/SoilTempForecastWidget.tsx](../../frontend/src/components/SoilTempForecastWidget.tsx)
- [frontend/src/components/dashboard/loadDashboard.ts](../../frontend/src/components/dashboard/loadDashboard.ts)
- [frontend/src/components/disease/DailyRiskChart.tsx](../../frontend/src/components/disease/DailyRiskChart.tsx)
- [frontend/src/components/disease/DiseaseRiskWidget.tsx](../../frontend/src/components/disease/DiseaseRiskWidget.tsx)
- [frontend/src/components/disease/TierBadge.tsx](../../frontend/src/components/disease/TierBadge.tsx)
- [frontend/src/components/inventory/ShoppingListPanel.tsx](../../frontend/src/components/inventory/ShoppingListPanel.tsx)
- [frontend/src/components/timing/SoilSeasonChart.tsx](../../frontend/src/components/timing/SoilSeasonChart.tsx)
- [frontend/src/components/timing/SoilSeasonTable.tsx](../../frontend/src/components/timing/SoilSeasonTable.tsx)
- [frontend/src/components/timing/TimingWidget.tsx](../../frontend/src/components/timing/TimingWidget.tsx)
- [frontend/src/components/useServerToday.ts](../../frontend/src/components/useServerToday.ts)
- [frontend/src/index.css](../../frontend/src/index.css)
- [frontend/src/main.tsx](../../frontend/src/main.tsx)
- [frontend/src/pages/Applications.tsx](../../frontend/src/pages/Applications.tsx)
- [frontend/src/pages/Calendar.tsx](../../frontend/src/pages/Calendar.tsx)
- [frontend/src/pages/Dashboard.tsx](../../frontend/src/pages/Dashboard.tsx)
- [frontend/src/pages/DiseaseRisk.tsx](../../frontend/src/pages/DiseaseRisk.tsx)
- [frontend/src/pages/Environmental.tsx](../../frontend/src/pages/Environmental.tsx)
- [frontend/src/pages/Inventory.tsx](../../frontend/src/pages/Inventory.tsx)
- [frontend/src/pages/Landscape.tsx](../../frontend/src/pages/Landscape.tsx)
- [frontend/src/pages/Recommendations.tsx](../../frontend/src/pages/Recommendations.tsx)
- [frontend/src/pages/SeasonalPlan.tsx](../../frontend/src/pages/SeasonalPlan.tsx)
- [frontend/src/pages/Settings.tsx](../../frontend/src/pages/Settings.tsx)
- [frontend/src/pages/SoilTests.tsx](../../frontend/src/pages/SoilTests.tsx)
- [frontend/src/pages/Timing.tsx](../../frontend/src/pages/Timing.tsx)
- [frontend/src/styles/shared.ts](../../frontend/src/styles/shared.ts)
- [frontend/src/types/disease.ts](../../frontend/src/types/disease.ts)
- [frontend/src/types/index.ts](../../frontend/src/types/index.ts)
- [frontend/src/types/inventory.ts](../../frontend/src/types/inventory.ts)
- [frontend/src/types/timing.ts](../../frontend/src/types/timing.ts)
- [frontend/src/utils/dates.ts](../../frontend/src/utils/dates.ts)
- [frontend/src/utils/units.ts](../../frontend/src/utils/units.ts)
- [frontend/vite.config.ts](../../frontend/vite.config.ts)

## Architecture

**Purpose:** see the LLD's Context and Design Philosophy; this page carries pointers and coverage only.

**Spec facets:**
1. `UI-SHELL` — entry, routes, error boundary, not-found page, page loading
2. `UI-NAV` — sidebar
3. `UI-DASH` — dashboard data loading
4. `UI-API` — the typed API client
5. `UI-ERR` — human-readable errors, client and API error body
6. `UI-TYPE` — shared TypeScript mirror
7. `UI-UTIL` — shared date, unit and label helpers
8. `UI-CONV` — cross-cutting presentation conventions
9. `UI-BUILD` — build, lint and type-check tooling
10. `UI-SRV` — SPA serving, request limits and CORS

## Spec Coverage

Derived from the spec file and a `@spec` scan; regenerate, don't hand-edit.

| Facet | Spec IDs | Implemented | Deferred | Gaps | `@spec` in code | `@spec` in tests |
|---|---|---|---|---|---|---|
| SHELL | UI-SHELL-001 – UI-SHELL-007 | 7 | 0 | 0 | 0 | 1 |
| NAV | UI-NAV-001 – UI-NAV-003 | 3 | 0 | 0 | 1 | 0 |
| DASH | UI-DASH-001 – UI-DASH-003 | 3 | 0 | 0 | 1 | 1 |
| API | UI-API-001 – UI-API-006 | 6 | 0 | 0 | 3 | 3 |
| ERR | UI-ERR-001 – UI-ERR-011 | 11 | 0 | 0 | 9 | 8 |
| TYPE | UI-TYPE-001 – UI-TYPE-004 | 4 | 0 | 0 | 0 | 0 |
| UTIL | UI-UTIL-001 – UI-UTIL-006 | 6 | 0 | 0 | 2 | 2 |
| CONV | UI-CONV-001 – UI-CONV-005 | 5 | 0 | 0 | 1 | 1 |
| BUILD | UI-BUILD-001 – UI-BUILD-009 | 9 | 0 | 0 | 0 | 1 |
| SRV | UI-SRV-001 – UI-SRV-010 | 9 | 0 | 0 | 4 | 4 |
| **Total** | | **63** | **0** | **0** | **21** | **21** |

**Summary:** 63 of 63 active specs implemented; 0 deferred; 21 implemented specs cited by a test, 40 cited nowhere.

## Key Findings

1. **40 implemented specs have no `@spec` annotation** — the specs were written to describe existing code and never linked back: UI-SHELL-001/002/003/004/005/006; UI-NAV-001/002; UI-DASH-001/002; UI-API-001/002/004; UI-ERR-001/002; UI-TYPE-001/002/003/004; UI-UTIL-001/002/003/004; UI-CONV-001/002/004/005; UI-BUILD-001/002/003/004/005/006/008/009; UI-SRV-002/003/004/005/006.
2. **2 specs are annotated in code but cited by no test** — UI-NAV-003; UI-ERR-011.

## Work Required

### Must Fix
- None.

### Should Fix
1. Add `@spec` annotations (entry point + exercising test) for the 40 unlinked specs.
2. Annotate a test for the 2 code-only specs.

### Nice to Have
- Audit each `[x]` against the code and move the segment to `AUDITED`.
