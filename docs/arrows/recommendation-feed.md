# Arrow: recommendation-feed

Recommendation Feed & Answered State — leaf segment `FEED` under the root HLD.

## Status

**MAPPED** — overlay bootstrapped 2026-10-11 from the existing LID docs; not yet audited (`audited_sha: null`). 49 specs `[x]`, 1 `[D]`, 0 `[ ]`; 10 of 49 implemented specs carry a `@spec` annotation.

## References

### HLD
- [docs/high-level-design.md](../../docs/high-level-design.md) § System Design — component "Recommendation feed and state"

### LLD
- [docs/intent/recommendation-feed/recommendation-feed-design.md](../../docs/intent/recommendation-feed/recommendation-feed-design.md)

### EARS
- [docs/intent/recommendation-feed/recommendation-feed-specs.md](../../docs/intent/recommendation-feed/recommendation-feed-specs.md) (50 specs, prefix `FEED`)

### Tests
- [frontend/src/components/dashboard/loadDashboard.test.ts](../../frontend/src/components/dashboard/loadDashboard.test.ts)
- [frontend/tests/repo.test.ts](../../frontend/tests/repo.test.ts)
- Inline `#[cfg(test)]` modules citing this prefix: `backend/src/api/recommendation_feed.rs`, `backend/src/models/recommendation_state.rs`

### Code
- [backend/src/api/dashboard.rs](../../backend/src/api/dashboard.rs)
- [backend/src/api/disease_risk.rs](../../backend/src/api/disease_risk.rs)
- [backend/src/api/inventory.rs](../../backend/src/api/inventory.rs)
- [backend/src/api/recommendation_feed.rs](../../backend/src/api/recommendation_feed.rs)
- [backend/src/api/recommendations.rs](../../backend/src/api/recommendations.rs)
- [backend/src/api/timing.rs](../../backend/src/api/timing.rs)
- [backend/src/db/migrations/20240101000003_recommendation_states.sql](../../backend/src/db/migrations/20240101000003_recommendation_states.sql)
- [backend/src/db/migrations/20260921000100_recommendation_state_severity.sql](../../backend/src/db/migrations/20260921000100_recommendation_state_severity.sql)
- [backend/src/db/queries.rs](../../backend/src/db/queries.rs)
- [backend/src/logic/inventory.rs](../../backend/src/logic/inventory.rs)
- [backend/src/logic/rules/nitrogen_guard.rs](../../backend/src/logic/rules/nitrogen_guard.rs)
- [backend/src/mcp/tools/lawn.rs](../../backend/src/mcp/tools/lawn.rs)
- [backend/src/models/recommendation.rs](../../backend/src/models/recommendation.rs)
- [backend/src/models/recommendation_state.rs](../../backend/src/models/recommendation_state.rs)
- [frontend/src/api/client.ts](../../frontend/src/api/client.ts)
- [frontend/src/components/AlertCard.tsx](../../frontend/src/components/AlertCard.tsx)
- [frontend/src/components/dashboard/loadDashboard.ts](../../frontend/src/components/dashboard/loadDashboard.ts)
- [frontend/src/pages/Dashboard.tsx](../../frontend/src/pages/Dashboard.tsx)
- [frontend/src/pages/Recommendations.tsx](../../frontend/src/pages/Recommendations.tsx)
- [frontend/src/types/index.ts](../../frontend/src/types/index.ts)

## Architecture

**Purpose:** see the LLD's Context and Design Philosophy; this page carries pointers and coverage only.

**Spec facets:**
1. `FEED-REC` — the recommendation contract
2. `FEED-PIPE` — building the feed
3. `FEED-STATE` — answered-state policy
4. `FEED-API` — endpoints
5. `FEED-UI` — Recommendations page and dashboard layout

## Spec Coverage

Derived from the spec file and a `@spec` scan; regenerate, don't hand-edit.

| Facet | Spec IDs | Implemented | Deferred | Gaps | `@spec` in code | `@spec` in tests |
|---|---|---|---|---|---|---|
| REC | FEED-REC-001 – FEED-REC-004 | 4 | 0 | 0 | 1 | 1 |
| PIPE | FEED-PIPE-001 – FEED-PIPE-018 | 16 | 1 | 0 | 6 | 3 |
| STATE | FEED-STATE-001 – FEED-STATE-010 | 10 | 0 | 0 | 1 | 1 |
| API | FEED-API-001 – FEED-API-006 | 6 | 0 | 0 | 1 | 0 |
| UI | FEED-UI-001 – FEED-UI-013 | 13 | 0 | 0 | 1 | 1 |
| **Total** | | **49** | **1** | **0** | **10** | **6** |

**Summary:** 49 of 49 active specs implemented; 1 deferred; 6 implemented specs cited by a test, 39 cited nowhere.

## Key Findings

1. **39 implemented specs have no `@spec` annotation** — the specs were written to describe existing code and never linked back: FEED-REC-001/002/003; FEED-PIPE-002/003/005/011/012/014/015/016/017/018; FEED-STATE-001/002/003/004/005/006/007/008/009; FEED-API-001/002/003/004/005; FEED-UI-001/002/003/004/005/006/007/008/009/010/011/012.
2. **4 specs are annotated in code but cited by no test** — FEED-PIPE-001/007/008; FEED-API-006.
3. **Deferred:** FEED-PIPE-010.

## Work Required

### Must Fix
- None.

### Should Fix
1. Add `@spec` annotations (entry point + exercising test) for the 39 unlinked specs.
2. Annotate a test for the 4 code-only specs.

### Nice to Have
- Audit each `[x]` against the code and move the segment to `AUDITED`.
