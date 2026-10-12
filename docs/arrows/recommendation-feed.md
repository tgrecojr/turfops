# Arrow: recommendation-feed

Recommendation Feed & Answered State — leaf segment `FEED` under the root HLD.

## Status

**OK** — audited 2026-10-12 (git SHA `2124c26`): every implemented spec was checked against the code while linking it. 48 of 49 implemented specs are linked to both code and a test.

## References

### HLD
- [docs/high-level-design.md](../../docs/high-level-design.md) § System Design — component "Recommendation feed and state"

### LLD
- [docs/intent/recommendation-feed/recommendation-feed-design.md](../../docs/intent/recommendation-feed/recommendation-feed-design.md)

### EARS
- [docs/intent/recommendation-feed/recommendation-feed-specs.md](../../docs/intent/recommendation-feed/recommendation-feed-specs.md) (50 specs, prefix `FEED`)

### Tests
- [backend/src/api/recommendation_feed/feed_db_tests.rs](../../backend/src/api/recommendation_feed/feed_db_tests.rs)
- [backend/src/api/recommendation_feed/outage_tests.rs](../../backend/src/api/recommendation_feed/outage_tests.rs)
- [backend/src/api/recommendation_feed/shared_log_db_tests.rs](../../backend/src/api/recommendation_feed/shared_log_db_tests.rs)
- [backend/src/api/recommendation_feed/stage_tests.rs](../../backend/src/api/recommendation_feed/stage_tests.rs)
- [backend/src/api/recommendations/db_tests.rs](../../backend/src/api/recommendations/db_tests.rs)
- [backend/src/models/recommendation/contract_tests.rs](../../backend/src/models/recommendation/contract_tests.rs)
- [backend/src/models/recommendation_state/policy_tests.rs](../../backend/src/models/recommendation_state/policy_tests.rs)
- [frontend/src/components/dashboard/loadDashboard.test.ts](../../frontend/src/components/dashboard/loadDashboard.test.ts)
- [frontend/src/pages/Dashboard.test.tsx](../../frontend/src/pages/Dashboard.test.tsx)
- [frontend/src/pages/Recommendations.test.tsx](../../frontend/src/pages/Recommendations.test.tsx)
- [frontend/tests/repo.test.ts](../../frontend/tests/repo.test.ts)
- Inline `#[cfg(test)]` modules citing this prefix: `backend/src/api/recommendation_feed.rs`, `backend/src/logic/rules/spring_nitrogen.rs`, `backend/src/models/recommendation_state.rs`

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
- [backend/src/logic/plant_maintenance.rs](../../backend/src/logic/plant_maintenance.rs)
- [backend/src/logic/rules/nitrogen_guard.rs](../../backend/src/logic/rules/nitrogen_guard.rs)
- [backend/src/logic/rules/spring_nitrogen.rs](../../backend/src/logic/rules/spring_nitrogen.rs)
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
| REC | FEED-REC-001 – FEED-REC-004 | 4 | 0 | 0 | 4 | 4 |
| PIPE | FEED-PIPE-001 – FEED-PIPE-018 | 16 | 1 | 0 | 16 | 15 |
| STATE | FEED-STATE-001 – FEED-STATE-010 | 10 | 0 | 0 | 10 | 10 |
| API | FEED-API-001 – FEED-API-006 | 6 | 0 | 0 | 6 | 6 |
| UI | FEED-UI-001 – FEED-UI-013 | 13 | 0 | 0 | 13 | 13 |
| **Total** | | **49** | **1** | **0** | **49** | **48** |

**Summary:** 49 of 49 active specs implemented; 1 deferred; 48 implemented specs cited by a test, 0 cited nowhere.

## Key Findings

1. **No test possible — FEED-PIPE-003:** timing-after-rules ordering is not observable: it needs a lake plus a rules-produced `broadleaf_fall`, both tied to the real date (`Clock::today()` reads `Utc::now()`).
2. **Deferred:** FEED-PIPE-010.

## Work Required

### Must Fix
- None.

### Should Fix
- None.

### Nice to Have
- FEED-PIPE-003: timing-after-rules ordering is not observable: it needs a lake plus a rules-produced `broadleaf_fall`, both tied to the real date (`Clock::today()` reads `Utc::now()`).
