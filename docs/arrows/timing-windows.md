# Arrow: timing-windows

Seeding & Pre-Emergent Timing Windows — leaf segment `TIME` under the root HLD.

## Status

**MAPPED** — overlay bootstrapped 2026-10-11 from the existing LID docs; not yet audited (`audited_sha: null`). 74 specs `[x]`, 0 `[D]`, 0 `[ ]`; 8 of 74 implemented specs carry a `@spec` annotation.

## References

### HLD
- [docs/high-level-design.md](../../docs/high-level-design.md) § System Design — component "Timing windows"

### LLD
- [docs/intent/timing-windows/timing-windows-design.md](../../docs/intent/timing-windows/timing-windows-design.md)

### EARS
- [docs/intent/timing-windows/timing-windows-specs.md](../../docs/intent/timing-windows/timing-windows-specs.md) (74 specs, prefix `TIME`)

### Related docs
- [docs/agronomy-methods.md](../../docs/agronomy-methods.md)
- [docs/plans/open-meteo-forecast.md](../../docs/plans/open-meteo-forecast.md)
- [docs/timing-windows.md](../../docs/timing-windows.md)

### Tests
- [backend/src/api/timing/memo_tests.rs](../../backend/src/api/timing/memo_tests.rs)
- Inline `#[cfg(test)]` modules citing this prefix: `backend/src/api/timing.rs`, `backend/src/logic/timing/evaluate.rs`, `backend/src/logic/timing/recommendations.rs`

### Code
- [backend/src/api/seasonal_plan.rs](../../backend/src/api/seasonal_plan.rs)
- [backend/src/api/timing.rs](../../backend/src/api/timing.rs)
- [backend/src/api/timing/memo.rs](../../backend/src/api/timing/memo.rs)
- [backend/src/datasources/weather/climatology.rs](../../backend/src/datasources/weather/climatology.rs)
- [backend/src/logic/data_sync.rs](../../backend/src/logic/data_sync.rs)
- [backend/src/logic/timing](../../backend/src/logic/timing)
- [backend/src/logic/timing/evaluate.rs](../../backend/src/logic/timing/evaluate.rs)
- [backend/src/logic/timing/recommendations.rs](../../backend/src/logic/timing/recommendations.rs)
- [backend/src/models/timing.rs](../../backend/src/models/timing.rs)
- [frontend/src/components/timing](../../frontend/src/components/timing)
- [frontend/src/pages/Timing.tsx](../../frontend/src/pages/Timing.tsx)
- [frontend/src/types/timing.ts](../../frontend/src/types/timing.ts)

## Architecture

**Purpose:** see the LLD's Context and Design Philosophy; this page carries pointers and coverage only.

## Spec Coverage

Derived from the spec file and a `@spec` scan; regenerate, don't hand-edit.

| Facet | Spec IDs | Implemented | Deferred | Gaps | `@spec` in code | `@spec` in tests |
|---|---|---|---|---|---|---|
| ENG | TIME-ENG-001 – TIME-ENG-046 | 46 | 0 | 0 | 3 | 3 |
| FEED | TIME-FEED-001 – TIME-FEED-009 | 9 | 0 | 0 | 3 | 3 |
| LOG | TIME-LOG-001 – TIME-LOG-004 | 4 | 0 | 0 | 0 | 0 |
| PLAN | TIME-PLAN-001 – TIME-PLAN-007 | 7 | 0 | 0 | 1 | 0 |
| UI | TIME-UI-001 – TIME-UI-008 | 8 | 0 | 0 | 1 | 0 |
| **Total** | | **74** | **0** | **0** | **8** | **6** |

**Summary:** 74 of 74 active specs implemented; 0 deferred; 6 implemented specs cited by a test, 66 cited nowhere.

## Key Findings

1. **66 implemented specs have no `@spec` annotation** — the specs were written to describe existing code and never linked back: TIME-ENG-001/002/003/004/005/006/007/008/009/010/011/012/013/014/015/016/017/018/019/020/021/022/023/024/025/026/027/029/030/031/032/033/034/035/036/037/038/040/041/042/043/044/045; TIME-FEED-001/002/004/005/006/007; TIME-LOG-001/002/003/004; TIME-PLAN-001/002/003/004/005/006; TIME-UI-001/002/003/004/005/006/007.
2. **2 specs are annotated in code but cited by no test** — TIME-PLAN-007; TIME-UI-008.

## Work Required

### Must Fix
- None.

### Should Fix
1. Add `@spec` annotations (entry point + exercising test) for the 66 unlinked specs.
2. Annotate a test for the 2 code-only specs.

### Nice to Have
- Audit each `[x]` against the code and move the segment to `AUDITED`.
