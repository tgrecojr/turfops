# Arrow: weather-sources

Weather Sources — leaf segment `WX-SRC` under the `weather-data` sub-HLD.

## Status

**MAPPED** — overlay bootstrapped 2026-10-11 from the existing LID docs; not yet audited (`audited_sha: null`). 28 specs `[x]`, 0 `[D]`, 0 `[ ]`; 7 of 28 implemented specs carry a `@spec` annotation.

## References

### HLD
- [docs/high-level-design.md](../../../docs/high-level-design.md) § System Design — component "Weather data access"
- [docs/intent/weather-data/weather-data-design.md](../../../docs/intent/weather-data/weather-data-design.md) (parent sub-HLD)

### LLD
- [docs/intent/weather-data/weather-sources/weather-sources-design.md](../../../docs/intent/weather-data/weather-sources/weather-sources-design.md)

### EARS
- [docs/intent/weather-data/weather-sources/weather-sources-specs.md](../../../docs/intent/weather-data/weather-sources/weather-sources-specs.md) (28 specs, prefix `WX-SRC`)

### Related docs
- [docs/agronomy-methods.md](../../../docs/agronomy-methods.md)
- [docs/plans/open-meteo-forecast.md](../../../docs/plans/open-meteo-forecast.md)
- [docs/plans/uscrn-soil-dropout-investigation.md](../../../docs/plans/uscrn-soil-dropout-investigation.md)
- [docs/weather-data-api-spec.md](../../../docs/weather-data-api-spec.md)

### Tests
- [backend/src/config/load_tests.rs](../../../backend/src/config/load_tests.rs)
- [backend/src/datasources/openweathermap/tests.rs](../../../backend/src/datasources/openweathermap/tests.rs)
- [backend/src/datasources/weather/tests.rs](../../../backend/src/datasources/weather/tests.rs)
- Inline `#[cfg(test)]` modules citing this prefix: `backend/src/error.rs`, `backend/src/models/forecast.rs`

### Code
- [backend/src/clock.rs](../../../backend/src/clock.rs)
- [backend/src/config.rs](../../../backend/src/config.rs)
- [backend/src/config/load.rs](../../../backend/src/config/load.rs)
- [backend/src/datasources/homeassistant.rs](../../../backend/src/datasources/homeassistant.rs)
- [backend/src/datasources/openweathermap.rs](../../../backend/src/datasources/openweathermap.rs)
- [backend/src/datasources/weather.rs](../../../backend/src/datasources/weather.rs)
- [backend/src/error.rs](../../../backend/src/error.rs)
- [backend/src/models/environmental.rs](../../../backend/src/models/environmental.rs)
- [backend/src/models/forecast.rs](../../../backend/src/models/forecast.rs)

## Architecture

**Purpose:** see the LLD's Context and Design Philosophy; this page carries pointers and coverage only.

**Spec facets:**
1. `WX-SRC-LAKE` — lake access and query contracts
2. `WX-SRC-HA` — Home Assistant reader
3. `WX-SRC-OWM` — OpenWeatherMap forecast and day helpers
4. `WX-SRC-MODEL` — environmental data model
5. `WX-SRC-CFG` — configuration

## Spec Coverage

Derived from the spec file and a `@spec` scan; regenerate, don't hand-edit.

| Facet | Spec IDs | Implemented | Deferred | Gaps | `@spec` in code | `@spec` in tests |
|---|---|---|---|---|---|---|
| LAKE | WX-SRC-LAKE-001 – WX-SRC-LAKE-014 | 14 | 0 | 0 | 2 | 2 |
| HA | WX-SRC-HA-001 – WX-SRC-HA-002 | 2 | 0 | 0 | 0 | 0 |
| OWM | WX-SRC-OWM-001 – WX-SRC-OWM-006 | 6 | 0 | 0 | 3 | 4 |
| MODEL | WX-SRC-MODEL-001 – WX-SRC-MODEL-003 | 3 | 0 | 0 | 0 | 0 |
| CFG | WX-SRC-CFG-001 – WX-SRC-CFG-003 | 3 | 0 | 0 | 1 | 1 |
| **Total** | | **28** | **0** | **0** | **6** | **7** |

**Summary:** 28 of 28 active specs implemented; 0 deferred; 7 implemented specs cited by a test, 21 cited nowhere.

## Key Findings

1. **21 implemented specs have no `@spec` annotation** — the specs were written to describe existing code and never linked back: WX-SRC-LAKE-001/002/003/004/005/006/007/008/009/010/011/012; WX-SRC-HA-001/002; WX-SRC-OWM-001/003; WX-SRC-MODEL-001/002/003; WX-SRC-CFG-001/002.

## Work Required

### Must Fix
- None.

### Should Fix
1. Add `@spec` annotations (entry point + exercising test) for the 21 unlinked specs.

### Nice to Have
- Audit each `[x]` against the code and move the segment to `AUDITED`.
