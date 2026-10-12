# Arrow: deployment

Deployment, Configuration & Operations — leaf segment `DEPLOY` under the root HLD.

## Status

**MAPPED** — overlay bootstrapped 2026-10-11 from the existing LID docs; not yet audited (`audited_sha: null`). 67 specs `[x]`, 0 `[D]`, 0 `[ ]`; 23 of 67 implemented specs carry a `@spec` annotation.

## References

### HLD
- [docs/high-level-design.md](../../docs/high-level-design.md) § System Design — component "Deployment"

### LLD
- [docs/intent/deployment/deployment-design.md](../../docs/intent/deployment/deployment-design.md)

### EARS
- [docs/intent/deployment/deployment-specs.md](../../docs/intent/deployment/deployment-specs.md) (67 specs, prefix `DEPLOY`)

### Related docs
- [docs/agronomy-methods.md](../../docs/agronomy-methods.md)
- [docs/plans/ai-assistant.md](../../docs/plans/ai-assistant.md)
- [docs/timing-windows.md](../../docs/timing-windows.md)

### Tests
- [backend/src/config/load_tests.rs](../../backend/src/config/load_tests.rs)
- [backend/src/db/pool/tests.rs](../../backend/src/db/pool/tests.rs)
- [backend/src/deployment_tests.rs](../../backend/src/deployment_tests.rs)
- [backend/src/health_probe/tests.rs](../../backend/src/health_probe/tests.rs)
- [frontend/src/api/client.test.ts](../../frontend/src/api/client.test.ts)
- Inline `#[cfg(test)]` modules citing this prefix: `backend/src/api/json_errors.rs`, `backend/src/api/today.rs`, `backend/src/clock.rs`, `backend/src/error.rs`, `backend/src/logging.rs`

### Code
- [./frontend](../.././frontend)
- [.github/workflows](../../.github/workflows)
- [backend/.cargo/audit.toml](../../backend/.cargo/audit.toml)
- [backend/.env.example](../../backend/.env.example)
- [backend/Cargo.toml](../../backend/Cargo.toml)
- [backend/osv-scanner.toml](../../backend/osv-scanner.toml)
- [backend/src/api/health.rs](../../backend/src/api/health.rs)
- [backend/src/api/json_errors.rs](../../backend/src/api/json_errors.rs)
- [backend/src/api/today.rs](../../backend/src/api/today.rs)
- [backend/src/clock.rs](../../backend/src/clock.rs)
- [backend/src/config.rs](../../backend/src/config.rs)
- [backend/src/config/load.rs](../../backend/src/config/load.rs)
- [backend/src/datasources/openweathermap.rs](../../backend/src/datasources/openweathermap.rs)
- [backend/src/db/migrations](../../backend/src/db/migrations)
- [backend/src/db/pool.rs](../../backend/src/db/pool.rs)
- [backend/src/error.rs](../../backend/src/error.rs)
- [backend/src/health_probe.rs](../../backend/src/health_probe.rs)
- [backend/src/logging.rs](../../backend/src/logging.rs)
- [backend/src/logic/data_sync.rs](../../backend/src/logic/data_sync.rs)
- [backend/src/main.rs](../../backend/src/main.rs)
- [backend/src/mcp/config.rs](../../backend/src/mcp/config.rs)
- [backend/src/mcp/resources.rs](../../backend/src/mcp/resources.rs)

## Architecture

**Purpose:** see the LLD's Context and Design Philosophy; this page carries pointers and coverage only.

**Spec facets:**
1. `DEPLOY-IMAGE` — the Docker image
2. `DEPLOY-COMPOSE` — the example Compose stack
3. `DEPLOY-CONFIG` — environment configuration surface
4. `DEPLOY-TZ` — the one "today" clock
5. `DEPLOY-START` — startup sequence and migrations
6. `DEPLOY-HTTP` — router assembly, middleware and the HTTP error contract
7. `DEPLOY-HEALTH` — the health endpoint
8. `DEPLOY-LOG` — logging and secret hygiene
9. `DEPLOY-CI` — CI, release and supply chain

## Spec Coverage

Derived from the spec file and a `@spec` scan; regenerate, don't hand-edit.

| Facet | Spec IDs | Implemented | Deferred | Gaps | `@spec` in code | `@spec` in tests |
|---|---|---|---|---|---|---|
| IMAGE | DEPLOY-IMAGE-001 – DEPLOY-IMAGE-010 | 10 | 0 | 0 | 1 | 3 |
| COMPOSE | DEPLOY-COMPOSE-001 – DEPLOY-COMPOSE-010 | 9 | 0 | 0 | 0 | 3 |
| CONFIG | DEPLOY-CONFIG-001 – DEPLOY-CONFIG-013 | 11 | 0 | 0 | 3 | 4 |
| TZ | DEPLOY-TZ-001 – DEPLOY-TZ-006 | 6 | 0 | 0 | 4 | 4 |
| START | DEPLOY-START-001 – DEPLOY-START-007 | 7 | 0 | 0 | 2 | 1 |
| HTTP | DEPLOY-HTTP-001 – DEPLOY-HTTP-011 | 9 | 0 | 0 | 4 | 4 |
| HEALTH | DEPLOY-HEALTH-001 – DEPLOY-HEALTH-003 | 3 | 0 | 0 | 0 | 0 |
| LOG | DEPLOY-LOG-001 – DEPLOY-LOG-003 | 3 | 0 | 0 | 1 | 2 |
| CI | DEPLOY-CI-001 – DEPLOY-CI-009 | 9 | 0 | 0 | 0 | 1 |
| **Total** | | **67** | **0** | **0** | **15** | **22** |

**Summary:** 67 of 67 active specs implemented; 0 deferred; 22 implemented specs cited by a test, 44 cited nowhere.

## Key Findings

1. **44 implemented specs have no `@spec` annotation** — the specs were written to describe existing code and never linked back: DEPLOY-IMAGE-001/002/003/004/005/006/007; DEPLOY-COMPOSE-001/002/003/004/005/006; DEPLOY-CONFIG-001/003/004/007/008/009/011; DEPLOY-TZ-001/002; DEPLOY-START-001/002/003/004/005; DEPLOY-HTTP-001/004/005/006/007; DEPLOY-HEALTH-001/002/003; DEPLOY-LOG-002; DEPLOY-CI-001/002/003/004/005/006/007/008.
2. **1 specs are annotated in code but cited by no test** — DEPLOY-START-006.

## Work Required

### Must Fix
- None.

### Should Fix
1. Add `@spec` annotations (entry point + exercising test) for the 44 unlinked specs.
2. Annotate a test for the 1 code-only specs.

### Nice to Have
- Audit each `[x]` against the code and move the segment to `AUDITED`.
