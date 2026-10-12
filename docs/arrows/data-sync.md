# Arrow: data-sync

Data Sync — leaf segment `SYNC` under the root HLD.

## Status

**MAPPED** — overlay bootstrapped 2026-10-11 from the existing LID docs; not yet audited (`audited_sha: null`). 31 specs `[x]`, 0 `[D]`, 0 `[ ]`; 24 of 31 implemented specs carry a `@spec` annotation.

## References

### HLD
- [docs/high-level-design.md](../../docs/high-level-design.md) § System Design — component "Data sync"

### LLD
- [docs/intent/data-sync/data-sync-design.md](../../docs/intent/data-sync/data-sync-design.md)

### EARS
- [docs/intent/data-sync/data-sync-specs.md](../../docs/intent/data-sync/data-sync-specs.md) (31 specs, prefix `SYNC`)

### Tests
- [backend/src/api/timing/memo_tests.rs](../../backend/src/api/timing/memo_tests.rs)
- [backend/src/logic/data_sync/tests.rs](../../backend/src/logic/data_sync/tests.rs)

### Code
- [backend/src/api/dashboard.rs](../../backend/src/api/dashboard.rs)
- [backend/src/api/environmental.rs](../../backend/src/api/environmental.rs)
- [backend/src/api/health.rs](../../backend/src/api/health.rs)
- [backend/src/api/historical.rs](../../backend/src/api/historical.rs)
- [backend/src/api/timing.rs](../../backend/src/api/timing.rs)
- [backend/src/api/timing/memo.rs](../../backend/src/api/timing/memo.rs)
- [backend/src/clock.rs](../../backend/src/clock.rs)
- [backend/src/datasources/homeassistant.rs](../../backend/src/datasources/homeassistant.rs)
- [backend/src/datasources/openweathermap.rs](../../backend/src/datasources/openweathermap.rs)
- [backend/src/datasources/weather.rs](../../backend/src/datasources/weather.rs)
- [backend/src/datasources/weather/latest.rs](../../backend/src/datasources/weather/latest.rs)
- [backend/src/logic/data_sync.rs](../../backend/src/logic/data_sync.rs)
- [backend/src/logic/data_sync/probes.rs](../../backend/src/logic/data_sync/probes.rs)
- [backend/src/logic/data_sync/sync_state.rs](../../backend/src/logic/data_sync/sync_state.rs)
- [backend/src/main.rs](../../backend/src/main.rs)
- [backend/src/models/environmental.rs](../../backend/src/models/environmental.rs)
- [backend/src/state.rs](../../backend/src/state.rs)
- [frontend/src/pages/Dashboard.tsx](../../frontend/src/pages/Dashboard.tsx)
- [frontend/src/pages/Environmental.tsx](../../frontend/src/pages/Environmental.tsx)

## Architecture

**Purpose:** see the LLD's Context and Design Philosophy; this page carries pointers and coverage only.

**Spec facets:**
1. `SYNC-REFRESH` — staleness limits and demand-driven refresh
2. `SYNC-MERGE` — assembling the cached environmental summary
3. `SYNC-FAIL` — failures, last-good values, the 72 h soil cut-off, forecast reuse, retry
4. `SYNC-CONN` — connection checks and `/health`
5. `SYNC-MEMO` — the shared climate-record slot in runtime state

## Spec Coverage

Derived from the spec file and a `@spec` scan; regenerate, don't hand-edit.

| Facet | Spec IDs | Implemented | Deferred | Gaps | `@spec` in code | `@spec` in tests |
|---|---|---|---|---|---|---|
| REFRESH | SYNC-REFRESH-001 – SYNC-REFRESH-008 | 8 | 0 | 0 | 4 | 2 |
| MERGE | SYNC-MERGE-001 – SYNC-MERGE-006 | 6 | 0 | 0 | 6 | 6 |
| FAIL | SYNC-FAIL-001 – SYNC-FAIL-009 | 9 | 0 | 0 | 9 | 8 |
| CONN | SYNC-CONN-001 – SYNC-CONN-006 | 6 | 0 | 0 | 3 | 1 |
| MEMO | SYNC-MEMO-001 – SYNC-MEMO-002 | 2 | 0 | 0 | 2 | 1 |
| **Total** | | **31** | **0** | **0** | **24** | **18** |

**Summary:** 31 of 31 active specs implemented; 0 deferred; 18 implemented specs cited by a test, 7 cited nowhere.

## Key Findings

1. **7 implemented specs have no `@spec` annotation** — the specs were written to describe existing code and never linked back: SYNC-REFRESH-001/005/006/007; SYNC-CONN-003/004/005.
2. **6 specs are annotated in code but cited by no test** — SYNC-REFRESH-003/004; SYNC-FAIL-001; SYNC-CONN-001/002; SYNC-MEMO-001.

## Work Required

### Must Fix
- None.

### Should Fix
1. Add `@spec` annotations (entry point + exercising test) for the 7 unlinked specs.
2. Annotate a test for the 6 code-only specs.

### Nice to Have
- Audit each `[x]` against the code and move the segment to `AUDITED`.
