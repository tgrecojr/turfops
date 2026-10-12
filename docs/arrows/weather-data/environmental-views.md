# Arrow: environmental-views

Environmental Views — leaf segment `WX-VIEW` under the `weather-data` sub-HLD.

## Status

**MAPPED** — overlay bootstrapped 2026-10-11 from the existing LID docs; not yet audited (`audited_sha: null`). 18 specs `[x]`, 0 `[D]`, 0 `[ ]`; 9 of 18 implemented specs carry a `@spec` annotation.

## References

### HLD
- [docs/high-level-design.md](../../../docs/high-level-design.md) § System Design — component "Weather data access"
- [docs/intent/weather-data/weather-data-design.md](../../../docs/intent/weather-data/weather-data-design.md) (parent sub-HLD)

### LLD
- [docs/intent/weather-data/environmental-views/environmental-views-design.md](../../../docs/intent/weather-data/environmental-views/environmental-views-design.md)

### EARS
- [docs/intent/weather-data/environmental-views/environmental-views-specs.md](../../../docs/intent/weather-data/environmental-views/environmental-views-specs.md) (18 specs, prefix `WX-VIEW`)

### Tests
- [backend/src/datasources/weather/tests.rs](../../../backend/src/datasources/weather/tests.rs)
- [frontend/src/components/environmental/display.test.ts](../../../frontend/src/components/environmental/display.test.ts)
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
| API | WX-VIEW-API-001 – WX-VIEW-API-006 | 6 | 0 | 0 | 2 | 2 |
| PROBE | WX-VIEW-PROBE-001 | 1 | 0 | 0 | 1 | 1 |
| PAGE | WX-VIEW-PAGE-001 – WX-VIEW-PAGE-011 | 11 | 0 | 0 | 6 | 5 |
| **Total** | | **18** | **0** | **0** | **9** | **8** |

**Summary:** 18 of 18 active specs implemented; 0 deferred; 8 implemented specs cited by a test, 9 cited nowhere.

## Key Findings

1. **9 implemented specs have no `@spec` annotation** — the specs were written to describe existing code and never linked back: WX-VIEW-API-001/002/004/006; WX-VIEW-PAGE-001/002/003/004/005.
2. **1 specs are annotated in code but cited by no test** — WX-VIEW-PAGE-006.

## Work Required

### Must Fix
- None.

### Should Fix
1. Add `@spec` annotations (entry point + exercising test) for the 9 unlinked specs.
2. Annotate a test for the 1 code-only specs.

### Nice to Have
- Audit each `[x]` against the code and move the segment to `AUDITED`.
