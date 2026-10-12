# Arrow: data-sync

Data Sync — leaf segment `SYNC` under the root HLD.

## Status

**OK** — audited 2026-10-12 (git SHA `4505fbc`): every implemented spec was checked against the code while linking it. 31 of 31 implemented specs are linked to both code and a test.

## References

### HLD
- [docs/high-level-design.md](../../docs/high-level-design.md) § System Design — component "Data sync"

### LLD
- [docs/intent/data-sync/data-sync-design.md](../../docs/intent/data-sync/data-sync-design.md)

### EARS
- [docs/intent/data-sync/data-sync-specs.md](../../docs/intent/data-sync/data-sync-specs.md) (31 specs, prefix `SYNC`)

### Tests
- [backend/src/api/health/db_tests.rs](../../backend/src/api/health/db_tests.rs)
- [backend/src/api/health/tests.rs](../../backend/src/api/health/tests.rs)
- [backend/src/api/timing/memo_tests.rs](../../backend/src/api/timing/memo_tests.rs)
- [backend/src/logic/data_sync/db_tests.rs](../../backend/src/logic/data_sync/db_tests.rs)
- [backend/src/logic/data_sync/memo_slot_tests.rs](../../backend/src/logic/data_sync/memo_slot_tests.rs)
- [backend/src/logic/data_sync/refresh_callers_tests.rs](../../backend/src/logic/data_sync/refresh_callers_tests.rs)
- [backend/src/logic/data_sync/service_tests.rs](../../backend/src/logic/data_sync/service_tests.rs)
- [backend/src/logic/data_sync/tests.rs](../../backend/src/logic/data_sync/tests.rs)

### Code
- [backend/src/api/dashboard.rs](../../backend/src/api/dashboard.rs)
- [backend/src/api/environmental.rs](../../backend/src/api/environmental.rs)
- [backend/src/api/health.rs](../../backend/src/api/health.rs)
- [backend/src/api/historical.rs](../../backend/src/api/historical.rs)
- [backend/src/api/seasonal_plan.rs](../../backend/src/api/seasonal_plan.rs)
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
- [backend/src/mcp/tools/lawn.rs](../../backend/src/mcp/tools/lawn.rs)
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
| REFRESH | SYNC-REFRESH-001 – SYNC-REFRESH-008 | 8 | 0 | 0 | 8 | 8 |
| MERGE | SYNC-MERGE-001 – SYNC-MERGE-006 | 6 | 0 | 0 | 6 | 6 |
| FAIL | SYNC-FAIL-001 – SYNC-FAIL-009 | 9 | 0 | 0 | 9 | 9 |
| CONN | SYNC-CONN-001 – SYNC-CONN-006 | 6 | 0 | 0 | 6 | 6 |
| MEMO | SYNC-MEMO-001 – SYNC-MEMO-002 | 2 | 0 | 0 | 2 | 2 |
| **Total** | | **31** | **0** | **0** | **31** | **31** |

**Summary:** 31 of 31 active specs implemented; 0 deferred; 31 implemented specs cited by a test, 0 cited nowhere.

## Key Findings

1. No findings.

## Work Required

### Must Fix
- None.

### Should Fix
- None.

### Nice to Have
- None.
