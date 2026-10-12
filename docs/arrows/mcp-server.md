# Arrow: mcp-server

MCP Server — leaf segment `MCP` under the root HLD.

## Status

**PARTIAL** — audited 2026-10-11 (git SHA `65245e3`): every implemented spec was checked against the code while linking it. 80 of 82 implemented specs are linked to both code and a test; 2 disagree with the code (see Key Findings).

## References

### HLD
- [docs/high-level-design.md](../../docs/high-level-design.md) § System Design — component "MCP server"

### LLD
- [docs/intent/mcp-server/mcp-server-design.md](../../docs/intent/mcp-server/mcp-server-design.md)

### EARS
- [docs/intent/mcp-server/mcp-server-specs.md](../../docs/intent/mcp-server/mcp-server-specs.md) (82 specs, prefix `MCP`)

### Related docs
- [docs/agronomy-methods.md](../../docs/agronomy-methods.md)
- [docs/plans/ai-assistant.md](../../docs/plans/ai-assistant.md)
- [docs/timing-windows.md](../../docs/timing-windows.md)

### Tests
- [backend/src/logic/data_sync/tests.rs](../../backend/src/logic/data_sync/tests.rs)
- [backend/src/mcp/doctrine_tests.rs](../../backend/src/mcp/doctrine_tests.rs)
- [backend/src/mcp/guard_tests.rs](../../backend/src/mcp/guard_tests.rs)
- [backend/src/mcp/mount_tests.rs](../../backend/src/mcp/mount_tests.rs)
- [backend/src/mcp/resource_tests.rs](../../backend/src/mcp/resource_tests.rs)
- [backend/src/mcp/server_tests.rs](../../backend/src/mcp/server_tests.rs)
- [backend/src/mcp/tools/error_tests.rs](../../backend/src/mcp/tools/error_tests.rs)
- [backend/src/mcp/tools/history_tests.rs](../../backend/src/mcp/tools/history_tests.rs)
- [backend/src/mcp/tools/log_db_tests.rs](../../backend/src/mcp/tools/log_db_tests.rs)
- [backend/src/mcp/tools/season_db_tests.rs](../../backend/src/mcp/tools/season_db_tests.rs)
- [backend/src/mcp/tools/shelf_db_tests.rs](../../backend/src/mcp/tools/shelf_db_tests.rs)
- [backend/src/mcp/tools/snapshot_db_tests.rs](../../backend/src/mcp/tools/snapshot_db_tests.rs)
- [backend/src/mcp/trim_model_tests.rs](../../backend/src/mcp/trim_model_tests.rs)
- Inline `#[cfg(test)]` modules citing this prefix: `backend/src/mcp/auth.rs`, `backend/src/mcp/config.rs`, `backend/src/mcp/mod.rs`, `backend/src/mcp/resources.rs`, `backend/src/mcp/tools/log.rs`, `backend/src/mcp/tools/mod.rs`, `backend/src/mcp/tools/shelf.rs`, `backend/src/mcp/tools/weather.rs`, `backend/src/mcp/trim.rs`

### Code
- [backend/src/api/health.rs](../../backend/src/api/health.rs)
- [backend/src/db/queries.rs](../../backend/src/db/queries.rs)
- [backend/src/error.rs](../../backend/src/error.rs)
- [backend/src/main.rs](../../backend/src/main.rs)
- [backend/src/mcp](../../backend/src/mcp)
- [backend/src/mcp/auth.rs](../../backend/src/mcp/auth.rs)
- [backend/src/mcp/config.rs](../../backend/src/mcp/config.rs)
- [backend/src/mcp/instructions.rs](../../backend/src/mcp/instructions.rs)
- [backend/src/mcp/mod.rs](../../backend/src/mcp/mod.rs)
- [backend/src/mcp/resources.rs](../../backend/src/mcp/resources.rs)
- [backend/src/mcp/tools/disease.rs](../../backend/src/mcp/tools/disease.rs)
- [backend/src/mcp/tools/lawn.rs](../../backend/src/mcp/tools/lawn.rs)
- [backend/src/mcp/tools/log.rs](../../backend/src/mcp/tools/log.rs)
- [backend/src/mcp/tools/mod.rs](../../backend/src/mcp/tools/mod.rs)
- [backend/src/mcp/tools/season.rs](../../backend/src/mcp/tools/season.rs)
- [backend/src/mcp/tools/shelf.rs](../../backend/src/mcp/tools/shelf.rs)
- [backend/src/mcp/tools/weather.rs](../../backend/src/mcp/tools/weather.rs)
- [backend/src/mcp/trim.rs](../../backend/src/mcp/trim.rs)

## Architecture

**Purpose:** see the LLD's Context and Design Philosophy; this page carries pointers and coverage only.

**Spec facets:**
1. `MCP-CFG` — `MCP_TOKEN` and mounting
2. `MCP-AUTH` — bearer guard
3. `MCP-SRV` — transport, server info, read-only contract
4. `MCP-TOOL` — tool catalog, handler mapping, parameters and trimming
5. `MCP-SNAP` — `lawn_snapshot`
6. `MCP-HIST` — `weather_history`
7. `MCP-ERR` — handler errors as tool errors
8. `MCP-DOC` — the doctrine returned at `initialize`
9. `MCP-RES` — methodology resources

## Spec Coverage

Derived from the spec file and a `@spec` scan; regenerate, don't hand-edit.

| Facet | Spec IDs | Implemented | Deferred | Gaps | `@spec` in code | `@spec` in tests |
|---|---|---|---|---|---|---|
| CFG | MCP-CFG-001 – MCP-CFG-006 | 6 | 0 | 0 | 6 | 6 |
| AUTH | MCP-AUTH-001 – MCP-AUTH-003 | 3 | 0 | 0 | 3 | 3 |
| SRV | MCP-SRV-001 – MCP-SRV-007 | 7 | 0 | 0 | 6 | 6 |
| TOOL | MCP-TOOL-001 – MCP-TOOL-027 | 27 | 0 | 0 | 27 | 27 |
| SNAP | MCP-SNAP-001 – MCP-SNAP-010 | 10 | 0 | 0 | 10 | 10 |
| HIST | MCP-HIST-001 – MCP-HIST-007 | 7 | 0 | 0 | 6 | 6 |
| ERR | MCP-ERR-001 – MCP-ERR-007 | 7 | 0 | 0 | 7 | 7 |
| DOC | MCP-DOC-001 – MCP-DOC-011 | 11 | 0 | 0 | 11 | 11 |
| RES | MCP-RES-001 – MCP-RES-004 | 4 | 0 | 0 | 4 | 4 |
| **Total** | | **82** | **0** | **0** | **80** | **80** |

**Summary:** 82 of 82 active specs implemented; 0 deferred; 80 implemented specs cited by a test, 2 cited nowhere.

## Key Findings

1. **Spec ≠ code — MCP-HIST-007:** spec quotes "Data source unavailable: weather data lake not configured."; code returns "Data source unavailable: " + the owner sentence, per MCP-ERR-006 (`mcp/tools/mod.rs` `error_message`) — spec stale.
2. **Spec ≠ code — MCP-SRV-005:** soft: `seasonal_plan`, `disease_risk`, `soil_tests`, `shopping_list`, `plants`, `gdd` (arguably `recommendations`) say what they return, not when to call them.

## Work Required

### Must Fix
- Decide MCP-HIST-007: fix the code or correct the spec, then link it.
- Decide MCP-SRV-005: fix the code or correct the spec, then link it.

### Should Fix
- None.

### Nice to Have
- None.
