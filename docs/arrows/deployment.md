# Arrow: deployment

Deployment, Configuration & Operations — leaf segment `DEPLOY` under the root HLD.

## Status

**OK** — audited 2026-10-12 (git SHA `4505fbc`): every implemented spec was checked against the code while linking it. 66 of 68 implemented specs are linked to both code and a test.

## References

### HLD
- [docs/high-level-design.md](../../docs/high-level-design.md) § System Design — component "Deployment"

### LLD
- [docs/intent/deployment/deployment-design.md](../../docs/intent/deployment/deployment-design.md)

### EARS
- [docs/intent/deployment/deployment-specs.md](../../docs/intent/deployment/deployment-specs.md) (68 specs, prefix `DEPLOY`)

### Related docs
- [docs/agronomy-methods.md](../../docs/agronomy-methods.md)
- [docs/plans/ai-assistant.md](../../docs/plans/ai-assistant.md)
- [docs/timing-windows.md](../../docs/timing-windows.md)

### Tests
- [backend/src/api/health/probe_tests.rs](../../backend/src/api/health/probe_tests.rs)
- [backend/src/config/defaults_tests.rs](../../backend/src/config/defaults_tests.rs)
- [backend/src/config/load_tests.rs](../../backend/src/config/load_tests.rs)
- [backend/src/db/pool/tests.rs](../../backend/src/db/pool/tests.rs)
- [backend/src/deployment_tests.rs](../../backend/src/deployment_tests.rs)
- [backend/src/deployment_tests/ci.rs](../../backend/src/deployment_tests/ci.rs)
- [backend/src/deployment_tests/compose.rs](../../backend/src/deployment_tests/compose.rs)
- [backend/src/deployment_tests/image.rs](../../backend/src/deployment_tests/image.rs)
- [backend/src/deployment_tests/shutdown.rs](../../backend/src/deployment_tests/shutdown.rs)
- [backend/src/deployment_tests/sources.rs](../../backend/src/deployment_tests/sources.rs)
- [backend/src/deployment_tests/startup.rs](../../backend/src/deployment_tests/startup.rs)
- [backend/src/deployment_tests/startup_db.rs](../../backend/src/deployment_tests/startup_db.rs)
- [backend/src/error/contract_tests.rs](../../backend/src/error/contract_tests.rs)
- [backend/src/error/log_level_tests.rs](../../backend/src/error/log_level_tests.rs)
- [backend/src/health_probe/tests.rs](../../backend/src/health_probe/tests.rs)
- [backend/src/web/tests.rs](../../backend/src/web/tests.rs)
- [frontend/src/api/client.test.ts](../../frontend/src/api/client.test.ts)
- Inline `#[cfg(test)]` modules citing this prefix: `backend/src/api/json_errors.rs`, `backend/src/api/today.rs`, `backend/src/clock.rs`, `backend/src/error.rs`, `backend/src/logging.rs`

### Code
- [./frontend](../.././frontend)
- [.dockerignore](../../.dockerignore)
- [.github/workflows](../../.github/workflows)
- [.github/workflows/ci.yml](../../.github/workflows/ci.yml)
- [.github/workflows/docker-publish.yml](../../.github/workflows/docker-publish.yml)
- [.github/workflows/ghcr-retention.yml](../../.github/workflows/ghcr-retention.yml)
- [.github/workflows/supply-chain.yml](../../.github/workflows/supply-chain.yml)
- [Dockerfile](../../Dockerfile)
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
- [backend/src/db/mod.rs](../../backend/src/db/mod.rs)
- [backend/src/db/pool.rs](../../backend/src/db/pool.rs)
- [backend/src/error.rs](../../backend/src/error.rs)
- [backend/src/health_probe.rs](../../backend/src/health_probe.rs)
- [backend/src/logging.rs](../../backend/src/logging.rs)
- [backend/src/logic/data_sync.rs](../../backend/src/logic/data_sync.rs)
- [backend/src/main.rs](../../backend/src/main.rs)
- [backend/src/mcp/config.rs](../../backend/src/mcp/config.rs)
- [backend/src/mcp/resources.rs](../../backend/src/mcp/resources.rs)
- [backend/src/web.rs](../../backend/src/web.rs)
- [docker-compose.yml](../../docker-compose.yml)

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
| IMAGE | DEPLOY-IMAGE-001 – DEPLOY-IMAGE-010 | 10 | 0 | 0 | 10 | 10 |
| COMPOSE | DEPLOY-COMPOSE-001 – DEPLOY-COMPOSE-010 | 9 | 0 | 0 | 9 | 9 |
| CONFIG | DEPLOY-CONFIG-001 – DEPLOY-CONFIG-013 | 11 | 0 | 0 | 11 | 11 |
| TZ | DEPLOY-TZ-001 – DEPLOY-TZ-006 | 6 | 0 | 0 | 6 | 6 |
| START | DEPLOY-START-001 – DEPLOY-START-007 | 7 | 0 | 0 | 7 | 7 |
| HTTP | DEPLOY-HTTP-001 – DEPLOY-HTTP-011 | 9 | 0 | 0 | 9 | 9 |
| HEALTH | DEPLOY-HEALTH-001 – DEPLOY-HEALTH-003 | 3 | 0 | 0 | 3 | 3 |
| LOG | DEPLOY-LOG-001 – DEPLOY-LOG-003 | 3 | 0 | 0 | 3 | 3 |
| CI | DEPLOY-CI-001 – DEPLOY-CI-010 | 10 | 0 | 0 | 8 | 9 |
| **Total** | | **68** | **0** | **0** | **66** | **67** |

**Summary:** 68 of 68 active specs implemented; 0 deferred; 67 implemented specs cited by a test, 1 cited nowhere.

## Key Findings

1. **No code anchor — DEPLOY-CI-007:** tested (`deployment_tests/ci.rs`); `renovate.json` cannot hold a comment.
2. **No code anchor — DEPLOY-CI-008:** enforced by GitHub repository settings and process; nothing in the repo to annotate or test.

## Work Required

### Must Fix
- None.

### Should Fix
- None.

### Nice to Have
- DEPLOY-CI-007: tested (`deployment_tests/ci.rs`); `renovate.json` cannot hold a comment.
- DEPLOY-CI-008: enforced by GitHub repository settings and process; nothing in the repo to annotate or test.
