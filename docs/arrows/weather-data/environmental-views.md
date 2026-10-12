# Arrow: environmental-views

Environmental Views — leaf segment `WX-VIEW` under the `weather-data` sub-HLD.

## Status

**OK** — audited 2026-10-12 (git SHA `2bf061e`): every implemented spec was checked against the code while linking it. 18 of 18 implemented specs are linked to both code and a test.

## References

### HLD
- [docs/high-level-design.md](../../../docs/high-level-design.md) § System Design — component "Weather data access"
- [docs/intent/weather-data/weather-data-design.md](../../../docs/intent/weather-data/weather-data-design.md) (parent sub-HLD)

### LLD
- [docs/intent/weather-data/environmental-views/environmental-views-design.md](../../../docs/intent/weather-data/environmental-views/environmental-views-design.md)

### EARS
- [docs/intent/weather-data/environmental-views/environmental-views-specs.md](../../../docs/intent/weather-data/environmental-views/environmental-views-specs.md) (18 specs, prefix `WX-VIEW`)

### Tests
- [backend/src/api/environmental/tests.rs](../../../backend/src/api/environmental/tests.rs)
- [backend/src/api/historical/handler_tests.rs](../../../backend/src/api/historical/handler_tests.rs)
- [backend/src/datasources/weather/tests.rs](../../../backend/src/datasources/weather/tests.rs)
- [frontend/src/components/environmental/display.test.ts](../../../frontend/src/components/environmental/display.test.ts)
- [frontend/src/pages/Environmental.test.tsx](../../../frontend/src/pages/Environmental.test.tsx)
- [frontend/src/pages/Environmental.trends.test.tsx](../../../frontend/src/pages/Environmental.trends.test.tsx)
- Inline `#[cfg(test)]` modules citing this prefix: `backend/src/api/historical.rs`

### Code
- [backend/src/api/environmental.rs](../../../backend/src/api/environmental.rs)
- [backend/src/api/historical.rs](../../../backend/src/api/historical.rs)
- [backend/src/datasources/weather.rs](../../../backend/src/datasources/weather.rs)
- [backend/src/models/historical.rs](../../../backend/src/models/historical.rs)
- [frontend/src/components/Gauge.tsx](../../../frontend/src/components/Gauge.tsx)
- [frontend/src/components/TrendChart.tsx](../../../frontend/src/components/TrendChart.tsx)
- [frontend/src/components/environmental/display.ts](../../../frontend/src/components/environmental/display.ts)
- [frontend/src/components/gaugeConfigs.ts](../../../frontend/src/components/gaugeConfigs.ts)
- [frontend/src/pages/Environmental.tsx](../../../frontend/src/pages/Environmental.tsx)
- [frontend/src/utils/units.ts](../../../frontend/src/utils/units.ts)

## Architecture

**Purpose:** see the LLD's Context and Design Philosophy; this page carries pointers and coverage only.

**Spec facets:**
1. `WX-VIEW-API` — `/environmental`, `/environmental/refresh`, `/historical`
2. `WX-VIEW-PROBE` — the lake connection probe
3. `WX-VIEW-PAGE` — the Environmental page, gauges and trend charts

## Spec Coverage

Derived from the spec file and a `@spec` scan; regenerate, don't hand-edit.

| Facet | Spec IDs | Implemented | Deferred | Gaps | `@spec` in code | `@spec` in tests |
|---|---|---|---|---|---|---|
| API | WX-VIEW-API-001 – WX-VIEW-API-006 | 6 | 0 | 0 | 6 | 6 |
| PROBE | WX-VIEW-PROBE-001 | 1 | 0 | 0 | 1 | 1 |
| PAGE | WX-VIEW-PAGE-001 – WX-VIEW-PAGE-011 | 11 | 0 | 0 | 11 | 11 |
| **Total** | | **18** | **0** | **0** | **18** | **18** |

**Summary:** 18 of 18 active specs implemented; 0 deferred; 18 implemented specs cited by a test, 0 cited nowhere.

## Key Findings

1. No findings.

## Work Required

### Must Fix
- None.

### Should Fix
- None.

### Nice to Have
- None.
