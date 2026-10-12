# Arrow: web-ui

Web UI Shell & Conventions — leaf segment `UI` under the root HLD.

## Status

**PARTIAL** — audited 2026-10-11 (git SHA `65245e3`): every implemented spec was checked against the code while linking it. 58 of 64 implemented specs are linked to both code and a test; 5 disagree with the code (see Key Findings).

## References

### HLD
- [docs/high-level-design.md](../../docs/high-level-design.md) § System Design — component "Web UI"

### LLD
- [docs/intent/web-ui/web-ui-design.md](../../docs/intent/web-ui/web-ui-design.md)

### EARS
- [docs/intent/web-ui/web-ui-specs.md](../../docs/intent/web-ui/web-ui-specs.md) (64 specs, prefix `UI`)

### Related docs
- [docs/timing-windows.md](../../docs/timing-windows.md)

### Tests
- [backend/src/config/cors_origin_tests.rs](../../backend/src/config/cors_origin_tests.rs)
- [backend/src/config/load_tests.rs](../../backend/src/config/load_tests.rs)
- [backend/src/error/error_body_tests.rs](../../backend/src/error/error_body_tests.rs)
- [backend/src/web/cors_tests.rs](../../backend/src/web/cors_tests.rs)
- [backend/src/web/limits_tests.rs](../../backend/src/web/limits_tests.rs)
- [backend/src/web/tests.rs](../../backend/src/web/tests.rs)
- [frontend/src/App.test.tsx](../../frontend/src/App.test.tsx)
- [frontend/src/api/client.contract.test.ts](../../frontend/src/api/client.contract.test.ts)
- [frontend/src/api/client.test.ts](../../frontend/src/api/client.test.ts)
- [frontend/src/api/errors.test.ts](../../frontend/src/api/errors.test.ts)
- [frontend/src/components/Layout.test.tsx](../../frontend/src/components/Layout.test.tsx)
- [frontend/src/components/dashboard/loadDashboard.test.ts](../../frontend/src/components/dashboard/loadDashboard.test.ts)
- [frontend/src/pages/DashboardLoad.test.tsx](../../frontend/src/pages/DashboardLoad.test.tsx)
- [frontend/src/pages/pageErrors.test.tsx](../../frontend/src/pages/pageErrors.test.tsx)
- [frontend/src/test/setup.test.tsx](../../frontend/src/test/setup.test.tsx)
- [frontend/src/types/mirror.test.ts](../../frontend/src/types/mirror.test.ts)
- [frontend/src/types/statusPalette.test.ts](../../frontend/src/types/statusPalette.test.ts)
- [frontend/src/utils/dates.test.ts](../../frontend/src/utils/dates.test.ts)
- [frontend/src/utils/helpers.test.ts](../../frontend/src/utils/helpers.test.ts)
- [frontend/tests/repo.test.ts](../../frontend/tests/repo.test.ts)
- [frontend/tests/shell.test.ts](../../frontend/tests/shell.test.ts)
- [frontend/tests/tooling.test.ts](../../frontend/tests/tooling.test.ts)
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
- [frontend/src/test/setup.ts](../../frontend/src/test/setup.ts)
- [frontend/src/types/disease.ts](../../frontend/src/types/disease.ts)
- [frontend/src/types/index.ts](../../frontend/src/types/index.ts)
- [frontend/src/types/inventory.ts](../../frontend/src/types/inventory.ts)
- [frontend/src/types/timing.ts](../../frontend/src/types/timing.ts)
- [frontend/src/utils/dates.ts](../../frontend/src/utils/dates.ts)
- [frontend/src/utils/units.ts](../../frontend/src/utils/units.ts)
- [frontend/tsconfig.app.json](../../frontend/tsconfig.app.json)
- [frontend/tsconfig.json](../../frontend/tsconfig.json)
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
| SHELL | UI-SHELL-001 – UI-SHELL-007 | 7 | 0 | 0 | 7 | 7 |
| NAV | UI-NAV-001 – UI-NAV-003 | 3 | 0 | 0 | 3 | 3 |
| DASH | UI-DASH-001 – UI-DASH-003 | 3 | 0 | 0 | 3 | 3 |
| API | UI-API-001 – UI-API-006 | 6 | 0 | 0 | 6 | 6 |
| ERR | UI-ERR-001 – UI-ERR-011 | 11 | 0 | 0 | 10 | 10 |
| TYPE | UI-TYPE-001 – UI-TYPE-004 | 4 | 0 | 0 | 3 | 3 |
| UTIL | UI-UTIL-001 – UI-UTIL-006 | 6 | 0 | 0 | 6 | 6 |
| CONV | UI-CONV-001 – UI-CONV-005 | 5 | 0 | 0 | 2 | 2 |
| BUILD | UI-BUILD-001 – UI-BUILD-010 | 10 | 0 | 0 | 9 | 10 |
| SRV | UI-SRV-001 – UI-SRV-010 | 9 | 0 | 0 | 9 | 9 |
| **Total** | | **64** | **0** | **0** | **58** | **59** |

**Summary:** 64 of 64 active specs implemented; 0 deferred; 59 implemented specs cited by a test, 5 cited nowhere.

## Key Findings

1. **No code anchor — UI-BUILD-003:** tested; `biome.json` / `package.json` cannot hold a comment.
2. **Spec ≠ code — UI-CONV-001:** the disease contributing-factor indicator shows symbol + note; its status word is only an `aria-label` and the symbol sits in an exposed `role="img"` span (`components/disease/DiseaseDetail.tsx:66-76`).
3. **Spec ≠ code — UI-CONV-004:** `SoilSeasonChart` marks today with an inline "Today" label; its legend has no entry for it (`components/timing/SoilSeasonChart.tsx:176-187`, legend 225-255).
4. **Spec ≠ code — UI-CONV-005:** `PredictionChart` draws observed soil `#e67e22` and the forecast `#3b82f6` dashed (a different hue), not `#2a78d6`/`#eb6834` with the forecast in the same hue (`components/PredictionChart.tsx:160-175`).
5. **Spec ≠ code — UI-ERR-002:** spec: a 500 says "An internal error occurred" and lake errors are 500; code: `SERVER_ERROR_MESSAGE` ("Something went wrong on the server…") and DataLake → 503 per WX-SRC-LAKE-014 (`backend/src/error.rs`) — spec stale.
6. **Spec ≠ code — UI-TYPE-004:** contradicts WX-OUTLOOK-UI-002: "the UI computes no agronomic result" vs `SoilTempForecastWidget.isMeaningfulCrossing`, which WX-OUTLOOK-UI-002 requires (`components/SoilTempForecastWidget.tsx:11-22`).

## Work Required

### Must Fix
- Decide UI-CONV-001: fix the code or correct the spec, then link it.
- Decide UI-CONV-004: fix the code or correct the spec, then link it.
- Decide UI-CONV-005: fix the code or correct the spec, then link it.
- Decide UI-ERR-002: fix the code or correct the spec, then link it.
- Decide UI-TYPE-004: fix the code or correct the spec, then link it.

### Should Fix
- None.

### Nice to Have
- UI-BUILD-003: tested; `biome.json` / `package.json` cannot hold a comment.
