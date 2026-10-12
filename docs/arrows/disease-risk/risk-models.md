# Arrow: risk-models

Disease Risk Models — leaf segment `DIS-MODEL` under the `disease-risk` sub-HLD.

## Status

**OK** — audited 2026-10-12 (git SHA `0e3be7a`): every implemented spec was checked against the code while linking it. 29 of 29 implemented specs are linked to both code and a test.

## References

### HLD
- [docs/high-level-design.md](../../../docs/high-level-design.md) § System Design — component "Disease risk"
- [docs/intent/disease-risk/disease-risk-design.md](../../../docs/intent/disease-risk/disease-risk-design.md) (parent sub-HLD)

### LLD
- [docs/intent/disease-risk/risk-models/risk-models-design.md](../../../docs/intent/disease-risk/risk-models/risk-models-design.md)

### EARS
- [docs/intent/disease-risk/risk-models/risk-models-specs.md](../../../docs/intent/disease-risk/risk-models/risk-models-specs.md) (29 specs, prefix `DIS-MODEL`)

### Related docs
- [docs/agronomy-methods.md](../../../docs/agronomy-methods.md)

### Tests
- [backend/src/api/disease_risk/db_tests.rs](../../../backend/src/api/disease_risk/db_tests.rs)
- [backend/src/logic/disease/amplifier_tests.rs](../../../backend/src/logic/disease/amplifier_tests.rs)
- [backend/src/logic/disease/score_tests.rs](../../../backend/src/logic/disease/score_tests.rs)
- [backend/src/logic/disease/window_tests.rs](../../../backend/src/logic/disease/window_tests.rs)
- Inline `#[cfg(test)]` modules citing this prefix: `backend/src/api/disease_risk.rs`, `backend/src/datasources/weather/disease.rs`, `backend/src/logic/disease/brown_patch.rs`, `backend/src/logic/disease/dollar_spot.rs`, `backend/src/logic/disease/gray_leaf_spot.rs`, `backend/src/logic/disease/history.rs`, `backend/src/logic/disease/mod.rs`, `backend/src/logic/disease/pythium.rs`, `backend/src/logic/disease/red_thread.rs`, `backend/src/logic/disease/weather_days.rs`

### Code
- [backend/src/api/disease_risk.rs](../../../backend/src/api/disease_risk.rs)
- [backend/src/datasources/weather/disease.rs](../../../backend/src/datasources/weather/disease.rs)
- [backend/src/logic/disease/brown_patch.rs](../../../backend/src/logic/disease/brown_patch.rs)
- [backend/src/logic/disease/dollar_spot.rs](../../../backend/src/logic/disease/dollar_spot.rs)
- [backend/src/logic/disease/gray_leaf_spot.rs](../../../backend/src/logic/disease/gray_leaf_spot.rs)
- [backend/src/logic/disease/history.rs](../../../backend/src/logic/disease/history.rs)
- [backend/src/logic/disease/mod.rs](../../../backend/src/logic/disease/mod.rs)
- [backend/src/logic/disease/pythium.rs](../../../backend/src/logic/disease/pythium.rs)
- [backend/src/logic/disease/red_thread.rs](../../../backend/src/logic/disease/red_thread.rs)
- [backend/src/logic/disease/weather_days.rs](../../../backend/src/logic/disease/weather_days.rs)
- [backend/src/models/disease.rs](../../../backend/src/models/disease.rs)

## Architecture

**Purpose:** see the LLD's Context and Design Philosophy; this page carries pointers and coverage only.

**Spec facets:**
1. `DIS-MODEL-DAY` — today, the daily series, usable days
2. `DIS-MODEL-HEAD` — headline day
3. `DIS-MODEL-SCORE` — the five models and tiers
4. `DIS-MODEL-AMP` — history amplifiers
5. `DIS-MODEL-API` — `GET /disease-risk` and data notes

## Spec Coverage

Derived from the spec file and a `@spec` scan; regenerate, don't hand-edit.

| Facet | Spec IDs | Implemented | Deferred | Gaps | `@spec` in code | `@spec` in tests |
|---|---|---|---|---|---|---|
| DAY | DIS-MODEL-DAY-001 – DIS-MODEL-DAY-009 | 9 | 0 | 0 | 9 | 9 |
| HEAD | DIS-MODEL-HEAD-001 – DIS-MODEL-HEAD-003 | 3 | 0 | 0 | 3 | 3 |
| SCORE | DIS-MODEL-SCORE-001 – DIS-MODEL-SCORE-008 | 8 | 0 | 0 | 8 | 8 |
| AMP | DIS-MODEL-AMP-001 – DIS-MODEL-AMP-004 | 4 | 0 | 0 | 4 | 4 |
| API | DIS-MODEL-API-001 – DIS-MODEL-API-005 | 5 | 0 | 0 | 5 | 5 |
| **Total** | | **29** | **0** | **0** | **29** | **29** |

**Summary:** 29 of 29 active specs implemented; 0 deferred; 29 implemented specs cited by a test, 0 cited nowhere.

## Key Findings

1. No findings.

## Work Required

### Must Fix
- None.

### Should Fix
- None.

### Nice to Have
- None.
