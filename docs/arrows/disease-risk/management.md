# Arrow: management

Disease Management — leaf segment `DIS-MGMT` under the `disease-risk` sub-HLD.

## Status

**MAPPED** — overlay bootstrapped 2026-10-11 from the existing LID docs; not yet audited (`audited_sha: null`). 35 specs `[x]`, 0 `[D]`, 0 `[ ]`; 6 of 35 implemented specs carry a `@spec` annotation.

## References

### HLD
- [docs/high-level-design.md](../../../docs/high-level-design.md) § System Design — component "Disease risk"
- [docs/intent/disease-risk/disease-risk-design.md](../../../docs/intent/disease-risk/disease-risk-design.md) (parent sub-HLD)

### LLD
- [docs/intent/disease-risk/management/management-design.md](../../../docs/intent/disease-risk/management/management-design.md)

### EARS
- [docs/intent/disease-risk/management/management-specs.md](../../../docs/intent/disease-risk/management/management-specs.md) (35 specs, prefix `DIS-MGMT`)

### Related docs
- [docs/agronomy-methods.md](../../../docs/agronomy-methods.md)

### Tests
- [backend/src/logic/disease/management/tests.rs](../../../backend/src/logic/disease/management/tests.rs)
- [backend/src/logic/disease/management/tests/outlook.rs](../../../backend/src/logic/disease/management/tests/outlook.rs)
- Inline `#[cfg(test)]` modules citing this prefix: `backend/src/logic/disease/recommendations.rs`

### Code
- [backend/src/api/disease_risk.rs](../../../backend/src/api/disease_risk.rs)
- [backend/src/api/frac_classes.rs](../../../backend/src/api/frac_classes.rs)
- [backend/src/logic/disease/history.rs](../../../backend/src/logic/disease/history.rs)
- [backend/src/logic/disease/management/headline.rs](../../../backend/src/logic/disease/management/headline.rs)
- [backend/src/logic/disease/management/mod.rs](../../../backend/src/logic/disease/management/mod.rs)
- [backend/src/logic/disease/management/programs.rs](../../../backend/src/logic/disease/management/programs.rs)
- [backend/src/logic/disease/recommendations.rs](../../../backend/src/logic/disease/recommendations.rs)
- [backend/src/models/disease_management.rs](../../../backend/src/models/disease_management.rs)
- [backend/src/models/frac_class.rs](../../../backend/src/models/frac_class.rs)
- [backend/src/models/frac_class/rotation.rs](../../../backend/src/models/frac_class/rotation.rs)

## Architecture

**Purpose:** see the LLD's Context and Design Philosophy; this page carries pointers and coverage only.

**Spec facets:**
1. `DIS-MGMT-ACT` — action and headline
2. `DIS-MGMT-PROT` — protection and the outlook
3. `DIS-MGMT-PICK` — recommended class
4. `DIS-MGMT-MATRIX` — efficacy matrix and programs
5. `DIS-MGMT-FRAC` — resolving logged fungicides, rotation
6. `DIS-MGMT-REC` — feed recommendations
7. `DIS-MGMT-API` — `GET /frac-classes`

## Spec Coverage

Derived from the spec file and a `@spec` scan; regenerate, don't hand-edit.

| Facet | Spec IDs | Implemented | Deferred | Gaps | `@spec` in code | `@spec` in tests |
|---|---|---|---|---|---|---|
| ACT | DIS-MGMT-ACT-001 – DIS-MGMT-ACT-006 | 6 | 0 | 0 | 0 | 0 |
| PROT | DIS-MGMT-PROT-001 – DIS-MGMT-PROT-008 | 8 | 0 | 0 | 2 | 2 |
| PICK | DIS-MGMT-PICK-001 – DIS-MGMT-PICK-005 | 5 | 0 | 0 | 1 | 1 |
| MATRIX | DIS-MGMT-MATRIX-001 – DIS-MGMT-MATRIX-004 | 4 | 0 | 0 | 0 | 0 |
| FRAC | DIS-MGMT-FRAC-001 – DIS-MGMT-FRAC-005 | 5 | 0 | 0 | 1 | 1 |
| REC | DIS-MGMT-REC-001 – DIS-MGMT-REC-006 | 6 | 0 | 0 | 2 | 2 |
| API | DIS-MGMT-API-001 | 1 | 0 | 0 | 0 | 0 |
| **Total** | | **35** | **0** | **0** | **6** | **6** |

**Summary:** 35 of 35 active specs implemented; 0 deferred; 6 implemented specs cited by a test, 29 cited nowhere.

## Key Findings

1. **29 implemented specs have no `@spec` annotation** — the specs were written to describe existing code and never linked back: DIS-MGMT-ACT-001/002/003/004/005/006; DIS-MGMT-PROT-001/002/003/004/005/006; DIS-MGMT-PICK-001/002/004/005; DIS-MGMT-MATRIX-001/002/003/004; DIS-MGMT-FRAC-001/002/003/004; DIS-MGMT-REC-001/002/003/005; DIS-MGMT-API-001.

## Work Required

### Must Fix
- None.

### Should Fix
1. Add `@spec` annotations (entry point + exercising test) for the 29 unlinked specs.

### Nice to Have
- Audit each `[x]` against the code and move the segment to `AUDITED`.
