# Arrow: risk-models

Disease Risk Models — leaf segment `DIS-MODEL` under the `disease-risk` sub-HLD.

## Status

**MAPPED** — overlay bootstrapped 2026-10-11 from the existing LID docs; not yet audited (`audited_sha: null`). 29 specs `[x]`, 0 `[D]`, 0 `[ ]`; 6 of 29 implemented specs carry a `@spec` annotation.

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
- Inline `#[cfg(test)]` modules citing this prefix: `backend/src/api/disease_risk.rs`, `backend/src/logic/disease/history.rs`, `backend/src/logic/disease/weather_days.rs`

### Code
- [backend/src/api/disease_risk.rs](../../../backend/src/api/disease_risk.rs)
- [backend/src/datasources/weather/disease.rs](../../../backend/src/datasources/weather/disease.rs)
- [backend/src/logic/disease/brown_patch.rs](../../../backend/src/logic/disease/brown_patch.rs)
- [backend/src/logic/disease/history.rs](../../../backend/src/logic/disease/history.rs)
- [backend/src/logic/disease/mod.rs](../../../backend/src/logic/disease/mod.rs)
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
| DAY | DIS-MODEL-DAY-001 – DIS-MODEL-DAY-009 | 9 | 0 | 0 | 3 | 2 |
| HEAD | DIS-MODEL-HEAD-001 – DIS-MODEL-HEAD-003 | 3 | 0 | 0 | 0 | 0 |
| SCORE | DIS-MODEL-SCORE-001 – DIS-MODEL-SCORE-008 | 8 | 0 | 0 | 0 | 0 |
| AMP | DIS-MODEL-AMP-001 – DIS-MODEL-AMP-004 | 4 | 0 | 0 | 1 | 1 |
| API | DIS-MODEL-API-001 – DIS-MODEL-API-005 | 5 | 0 | 0 | 2 | 2 |
| **Total** | | **29** | **0** | **0** | **6** | **5** |

**Summary:** 29 of 29 active specs implemented; 0 deferred; 5 implemented specs cited by a test, 23 cited nowhere.

## Key Findings

1. **23 implemented specs have no `@spec` annotation** — the specs were written to describe existing code and never linked back: DIS-MODEL-DAY-002/004/005/006/008/009; DIS-MODEL-HEAD-001/002/003; DIS-MODEL-SCORE-001/002/003/004/005/006/007/008; DIS-MODEL-AMP-002/003/004; DIS-MODEL-API-001/002/003.
2. **1 specs are annotated in code but cited by no test** — DIS-MODEL-DAY-001.

## Work Required

### Must Fix
- None.

### Should Fix
1. Add `@spec` annotations (entry point + exercising test) for the 23 unlinked specs.
2. Annotate a test for the 1 code-only specs.

### Nice to Have
- Audit each `[x]` against the code and move the segment to `AUDITED`.
