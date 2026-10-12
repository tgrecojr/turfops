# Arrow: soil-temp-outlook

Soil-Temperature Outlook — leaf segment `WX-OUTLOOK` under the `weather-data` sub-HLD.

## Status

**MAPPED** — overlay bootstrapped 2026-10-11 from the existing LID docs; not yet audited (`audited_sha: null`). 20 specs `[x]`, 0 `[D]`, 0 `[ ]`; 6 of 20 implemented specs carry a `@spec` annotation.

## References

### HLD
- [docs/high-level-design.md](../../../docs/high-level-design.md) § System Design — component "Weather data access"
- [docs/intent/weather-data/weather-data-design.md](../../../docs/intent/weather-data/weather-data-design.md) (parent sub-HLD)

### LLD
- [docs/intent/weather-data/soil-temp-outlook/soil-temp-outlook-design.md](../../../docs/intent/weather-data/soil-temp-outlook/soil-temp-outlook-design.md)

### EARS
- [docs/intent/weather-data/soil-temp-outlook/soil-temp-outlook-specs.md](../../../docs/intent/weather-data/soil-temp-outlook/soil-temp-outlook-specs.md) (20 specs, prefix `WX-OUTLOOK`)

### Related docs
- [docs/agronomy-methods.md](../../../docs/agronomy-methods.md)
- [docs/timing-windows.md](../../../docs/timing-windows.md)

### Tests
- [backend/src/logic/soil_temp_prediction/outlook_tests.rs](../../../backend/src/logic/soil_temp_prediction/outlook_tests.rs)
- [backend/src/logic/soil_temp_prediction/tests.rs](../../../backend/src/logic/soil_temp_prediction/tests.rs)
- [frontend/src/components/environmental/display.test.ts](../../../frontend/src/components/environmental/display.test.ts)
- Inline `#[cfg(test)]` modules citing this prefix: `backend/src/error.rs`

### Code
- [backend/src/api/soil_temp_prediction.rs](../../../backend/src/api/soil_temp_prediction.rs)
- [backend/src/error.rs](../../../backend/src/error.rs)
- [backend/src/logic/soil_temp_prediction.rs](../../../backend/src/logic/soil_temp_prediction.rs)
- [backend/src/logic/soil_temp_prediction/series.rs](../../../backend/src/logic/soil_temp_prediction/series.rs)
- [backend/src/logic/timing/mod.rs](../../../backend/src/logic/timing/mod.rs)
- [backend/src/models/soil_temp_prediction.rs](../../../backend/src/models/soil_temp_prediction.rs)
- [frontend/src/components/PredictionChart.tsx](../../../frontend/src/components/PredictionChart.tsx)
- [frontend/src/components/SoilTempForecastWidget.tsx](../../../frontend/src/components/SoilTempForecastWidget.tsx)
- [frontend/src/components/environmental/display.ts](../../../frontend/src/components/environmental/display.ts)

## Architecture

**Purpose:** see the LLD's Context and Design Philosophy; this page carries pointers and coverage only.

**Spec facets:**
1. `WX-OUTLOOK-MODEL` — regression and predictions, including the contract timing windows reuses
2. `WX-OUTLOOK-CROSS` — thresholds and crossings
3. `WX-OUTLOOK-API` — `GET /soil-temp-forecast`
4. `WX-OUTLOOK-UI` — `PredictionChart`, `SoilTempForecastWidget`

## Spec Coverage

Derived from the spec file and a `@spec` scan; regenerate, don't hand-edit.

| Facet | Spec IDs | Implemented | Deferred | Gaps | `@spec` in code | `@spec` in tests |
|---|---|---|---|---|---|---|
| MODEL | WX-OUTLOOK-MODEL-001 – WX-OUTLOOK-MODEL-008 | 8 | 0 | 0 | 3 | 3 |
| CROSS | WX-OUTLOOK-CROSS-001 – WX-OUTLOOK-CROSS-005 | 5 | 0 | 0 | 1 | 1 |
| API | WX-OUTLOOK-API-001 – WX-OUTLOOK-API-003 | 3 | 0 | 0 | 1 | 1 |
| UI | WX-OUTLOOK-UI-001 – WX-OUTLOOK-UI-004 | 4 | 0 | 0 | 1 | 1 |
| **Total** | | **20** | **0** | **0** | **6** | **6** |

**Summary:** 20 of 20 active specs implemented; 0 deferred; 6 implemented specs cited by a test, 14 cited nowhere.

## Key Findings

1. **14 implemented specs have no `@spec` annotation** — the specs were written to describe existing code and never linked back: WX-OUTLOOK-MODEL-002/003/004/005/006; WX-OUTLOOK-CROSS-001/002/003/004; WX-OUTLOOK-API-001/002; WX-OUTLOOK-UI-001/002/003.

## Work Required

### Must Fix
- None.

### Should Fix
1. Add `@spec` annotations (entry point + exercising test) for the 14 unlinked specs.

### Nice to Have
- Audit each `[x]` against the code and move the segment to `AUDITED`.
