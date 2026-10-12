# Arrow: soil-temp-outlook

Soil-Temperature Outlook — leaf segment `WX-OUTLOOK` under the `weather-data` sub-HLD.

## Status

**OK** — audited 2026-10-12 (git SHA `2bf061e`): every implemented spec was checked against the code while linking it. 21 of 21 implemented specs are linked to both code and a test.

## References

### HLD
- [docs/high-level-design.md](../../../docs/high-level-design.md) § System Design — component "Weather data access"
- [docs/intent/weather-data/weather-data-design.md](../../../docs/intent/weather-data/weather-data-design.md) (parent sub-HLD)

### LLD
- [docs/intent/weather-data/soil-temp-outlook/soil-temp-outlook-design.md](../../../docs/intent/weather-data/soil-temp-outlook/soil-temp-outlook-design.md)

### EARS
- [docs/intent/weather-data/soil-temp-outlook/soil-temp-outlook-specs.md](../../../docs/intent/weather-data/soil-temp-outlook/soil-temp-outlook-specs.md) (21 specs, prefix `WX-OUTLOOK`)

### Related docs
- [docs/agronomy-methods.md](../../../docs/agronomy-methods.md)
- [docs/timing-windows.md](../../../docs/timing-windows.md)

### Tests
- [backend/src/api/soil_temp_prediction/tests.rs](../../../backend/src/api/soil_temp_prediction/tests.rs)
- [backend/src/logic/soil_temp_prediction/crossing_tests.rs](../../../backend/src/logic/soil_temp_prediction/crossing_tests.rs)
- [backend/src/logic/soil_temp_prediction/dip_tests.rs](../../../backend/src/logic/soil_temp_prediction/dip_tests.rs)
- [backend/src/logic/soil_temp_prediction/model_tests.rs](../../../backend/src/logic/soil_temp_prediction/model_tests.rs)
- [backend/src/logic/soil_temp_prediction/outlook_tests.rs](../../../backend/src/logic/soil_temp_prediction/outlook_tests.rs)
- [backend/src/logic/soil_temp_prediction/tests.rs](../../../backend/src/logic/soil_temp_prediction/tests.rs)
- [frontend/src/components/PredictionChart.test.tsx](../../../frontend/src/components/PredictionChart.test.tsx)
- [frontend/src/components/SoilTempForecastWidget.test.tsx](../../../frontend/src/components/SoilTempForecastWidget.test.tsx)
- [frontend/src/components/environmental/display.test.ts](../../../frontend/src/components/environmental/display.test.ts)
- [frontend/src/pages/Dashboard.soilWidget.test.tsx](../../../frontend/src/pages/Dashboard.soilWidget.test.tsx)
- [frontend/src/pages/Environmental.test.tsx](../../../frontend/src/pages/Environmental.test.tsx)
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
- [frontend/src/pages/Dashboard.tsx](../../../frontend/src/pages/Dashboard.tsx)
- [frontend/src/pages/Environmental.tsx](../../../frontend/src/pages/Environmental.tsx)

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
| MODEL | WX-OUTLOOK-MODEL-001 – WX-OUTLOOK-MODEL-008 | 8 | 0 | 0 | 8 | 8 |
| CROSS | WX-OUTLOOK-CROSS-001 – WX-OUTLOOK-CROSS-006 | 6 | 0 | 0 | 6 | 6 |
| API | WX-OUTLOOK-API-001 – WX-OUTLOOK-API-003 | 3 | 0 | 0 | 3 | 3 |
| UI | WX-OUTLOOK-UI-001 – WX-OUTLOOK-UI-004 | 4 | 0 | 0 | 4 | 4 |
| **Total** | | **21** | **0** | **0** | **21** | **21** |

**Summary:** 21 of 21 active specs implemented; 0 deferred; 21 implemented specs cited by a test, 0 cited nowhere.

## Key Findings

1. No findings.

## Work Required

### Must Fix
- None.

### Should Fix
- None.

### Nice to Have
- None.
