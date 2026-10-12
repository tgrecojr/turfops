# Arrow: disease-ui

Disease Risk UI — leaf segment `DIS-UI` under the `disease-risk` sub-HLD.

## Status

**MAPPED** — overlay bootstrapped 2026-10-11 from the existing LID docs; not yet audited (`audited_sha: null`). 18 specs `[x]`, 0 `[D]`, 0 `[ ]`; 1 of 18 implemented specs carry a `@spec` annotation.

## References

### HLD
- [docs/high-level-design.md](../../../docs/high-level-design.md) § System Design — component "Disease risk"
- [docs/intent/disease-risk/disease-risk-design.md](../../../docs/intent/disease-risk/disease-risk-design.md) (parent sub-HLD)

### LLD
- [docs/intent/disease-risk/disease-ui/disease-ui-design.md](../../../docs/intent/disease-risk/disease-ui/disease-ui-design.md)

### EARS
- [docs/intent/disease-risk/disease-ui/disease-ui-specs.md](../../../docs/intent/disease-risk/disease-ui/disease-ui-specs.md) (18 specs, prefix `DIS-UI`)

### Tests
- *(no test cites this prefix yet)*

### Code
- [frontend/src/components/disease](../../../frontend/src/components/disease)
- [frontend/src/pages/Dashboard.tsx](../../../frontend/src/pages/Dashboard.tsx)
- [frontend/src/pages/DiseaseRisk.tsx](../../../frontend/src/pages/DiseaseRisk.tsx)
- [frontend/src/types/disease.ts](../../../frontend/src/types/disease.ts)

## Architecture

**Purpose:** see the LLD's Context and Design Philosophy; this page carries pointers and coverage only.

**Spec facets:**
1. `DIS-UI-PAGE` — list and detail routes
2. `DIS-UI-DETAIL` — detail view and management panel
3. `DIS-UI-CHART` — daily chart and meter
4. `DIS-UI-WIDGET` — dashboard
5. `DIS-UI-CONV` — tier and factor conventions

## Spec Coverage

Derived from the spec file and a `@spec` scan; regenerate, don't hand-edit.

| Facet | Spec IDs | Implemented | Deferred | Gaps | `@spec` in code | `@spec` in tests |
|---|---|---|---|---|---|---|
| PAGE | DIS-UI-PAGE-001 – DIS-UI-PAGE-005 | 5 | 0 | 0 | 1 | 0 |
| DETAIL | DIS-UI-DETAIL-001 – DIS-UI-DETAIL-005 | 5 | 0 | 0 | 0 | 0 |
| CHART | DIS-UI-CHART-001 – DIS-UI-CHART-004 | 4 | 0 | 0 | 0 | 0 |
| WIDGET | DIS-UI-WIDGET-001 | 1 | 0 | 0 | 0 | 0 |
| CONV | DIS-UI-CONV-001 – DIS-UI-CONV-003 | 3 | 0 | 0 | 0 | 0 |
| **Total** | | **18** | **0** | **0** | **1** | **0** |

**Summary:** 18 of 18 active specs implemented; 0 deferred; 0 implemented specs cited by a test, 17 cited nowhere.

## Key Findings

1. **17 implemented specs have no `@spec` annotation** — the specs were written to describe existing code and never linked back: DIS-UI-PAGE-001/002/003/004; DIS-UI-DETAIL-001/002/003/004/005; DIS-UI-CHART-001/002/003/004; DIS-UI-WIDGET-001; DIS-UI-CONV-001/002/003.
2. **1 specs are annotated in code but cited by no test** — DIS-UI-PAGE-005.

## Work Required

### Must Fix
- None.

### Should Fix
1. Add `@spec` annotations (entry point + exercising test) for the 17 unlinked specs.
2. Annotate a test for the 1 code-only specs.

### Nice to Have
- Audit each `[x]` against the code and move the segment to `AUDITED`.
