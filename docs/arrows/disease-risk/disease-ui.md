# Arrow: disease-ui

Disease Risk UI — leaf segment `DIS-UI` under the `disease-risk` sub-HLD.

## Status

**OK** — audited 2026-10-12 (git SHA `2124c26`): every implemented spec was checked against the code while linking it. 18 of 18 implemented specs are linked to both code and a test.

## References

### HLD
- [docs/high-level-design.md](../../../docs/high-level-design.md) § System Design — component "Disease risk"
- [docs/intent/disease-risk/disease-risk-design.md](../../../docs/intent/disease-risk/disease-risk-design.md) (parent sub-HLD)

### LLD
- [docs/intent/disease-risk/disease-ui/disease-ui-design.md](../../../docs/intent/disease-risk/disease-ui/disease-ui-design.md)

### EARS
- [docs/intent/disease-risk/disease-ui/disease-ui-specs.md](../../../docs/intent/disease-risk/disease-ui/disease-ui-specs.md) (18 specs, prefix `DIS-UI`)

### Tests
- [frontend/src/components/disease/DailyRiskChart.test.tsx](../../../frontend/src/components/disease/DailyRiskChart.test.tsx)
- [frontend/src/components/disease/DiseaseDetail.test.tsx](../../../frontend/src/components/disease/DiseaseDetail.test.tsx)
- [frontend/src/components/disease/DiseaseRiskWidget.test.tsx](../../../frontend/src/components/disease/DiseaseRiskWidget.test.tsx)
- [frontend/src/components/disease/ManagementPanel.test.tsx](../../../frontend/src/components/disease/ManagementPanel.test.tsx)
- [frontend/src/components/disease/RiskMeter.test.tsx](../../../frontend/src/components/disease/RiskMeter.test.tsx)
- [frontend/src/components/disease/TierBadge.test.tsx](../../../frontend/src/components/disease/TierBadge.test.tsx)
- [frontend/src/pages/DiseaseRisk.test.tsx](../../../frontend/src/pages/DiseaseRisk.test.tsx)

### Code
- [frontend/src/components/disease](../../../frontend/src/components/disease)
- [frontend/src/components/disease/DailyRiskChart.tsx](../../../frontend/src/components/disease/DailyRiskChart.tsx)
- [frontend/src/components/disease/DiseaseDetail.tsx](../../../frontend/src/components/disease/DiseaseDetail.tsx)
- [frontend/src/components/disease/DiseaseRiskWidget.tsx](../../../frontend/src/components/disease/DiseaseRiskWidget.tsx)
- [frontend/src/components/disease/ManagementPanel.tsx](../../../frontend/src/components/disease/ManagementPanel.tsx)
- [frontend/src/components/disease/RiskMeter.tsx](../../../frontend/src/components/disease/RiskMeter.tsx)
- [frontend/src/components/disease/TierBadge.tsx](../../../frontend/src/components/disease/TierBadge.tsx)
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
| PAGE | DIS-UI-PAGE-001 – DIS-UI-PAGE-005 | 5 | 0 | 0 | 5 | 5 |
| DETAIL | DIS-UI-DETAIL-001 – DIS-UI-DETAIL-005 | 5 | 0 | 0 | 5 | 5 |
| CHART | DIS-UI-CHART-001 – DIS-UI-CHART-004 | 4 | 0 | 0 | 4 | 4 |
| WIDGET | DIS-UI-WIDGET-001 | 1 | 0 | 0 | 1 | 1 |
| CONV | DIS-UI-CONV-001 – DIS-UI-CONV-003 | 3 | 0 | 0 | 3 | 3 |
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
