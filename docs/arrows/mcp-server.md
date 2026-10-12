# Arrow: mcp-server

MCP Server — leaf segment `MCP` under the root HLD.

## Status

**MAPPED** — overlay bootstrapped 2026-10-11 from the existing LID docs; not yet audited (`audited_sha: null`). 82 specs `[x]`, 0 `[D]`, 0 `[ ]`; 15 of 82 implemented specs carry a `@spec` annotation.

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
- [backend/src/mcp/guard_tests.rs](../../backend/src/mcp/guard_tests.rs)
- [backend/src/mcp/trim_model_tests.rs](../../backend/src/mcp/trim_model_tests.rs)
- Inline `#[cfg(test)]` modules citing this prefix: `backend/src/mcp/mod.rs`, `backend/src/mcp/tools/log.rs`, `backend/src/mcp/tools/mod.rs`, `backend/src/mcp/tools/shelf.rs`, `backend/src/mcp/tools/weather.rs`, `backend/src/mcp/trim.rs`

### Code
- [backend/src/db/queries.rs](../../backend/src/db/queries.rs)
- [backend/src/error.rs](../../backend/src/error.rs)
- [backend/src/mcp](../../backend/src/mcp)
- [backend/src/mcp/instructions.rs](../../backend/src/mcp/instructions.rs)
- [backend/src/mcp/mod.rs](../../backend/src/mcp/mod.rs)
- [backend/src/mcp/tools/log.rs](../../backend/src/mcp/tools/log.rs)
- [backend/src/mcp/tools/mod.rs](../../backend/src/mcp/tools/mod.rs)
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
| CFG | MCP-CFG-001 – MCP-CFG-006 | 6 | 0 | 0 | 0 | 0 |
| AUTH | MCP-AUTH-001 – MCP-AUTH-003 | 3 | 0 | 0 | 0 | 0 |
| SRV | MCP-SRV-001 – MCP-SRV-007 | 7 | 0 | 0 | 2 | 2 |
| TOOL | MCP-TOOL-001 – MCP-TOOL-027 | 27 | 0 | 0 | 4 | 4 |
| SNAP | MCP-SNAP-001 – MCP-SNAP-010 | 10 | 0 | 0 | 1 | 2 |
| HIST | MCP-HIST-001 – MCP-HIST-007 | 7 | 0 | 0 | 1 | 1 |
| ERR | MCP-ERR-001 – MCP-ERR-007 | 7 | 0 | 0 | 2 | 2 |
| DOC | MCP-DOC-001 – MCP-DOC-011 | 11 | 0 | 0 | 3 | 3 |
| RES | MCP-RES-001 – MCP-RES-004 | 4 | 0 | 0 | 0 | 0 |
| **Total** | | **82** | **0** | **0** | **13** | **14** |

**Summary:** 82 of 82 active specs implemented; 0 deferred; 14 implemented specs cited by a test, 67 cited nowhere.

## Key Findings

1. **67 implemented specs have no `@spec` annotation** — the specs were written to describe existing code and never linked back: MCP-CFG-001/002/003/004/005/006; MCP-AUTH-001/002/003; MCP-SRV-001/002/003/004/005; MCP-TOOL-001/002/003/004/005/006/007/008/009/010/011/012/013/014/015/016/017/019/020/021/024/025; MCP-SNAP-001/002/003/004/005/006/007/008; MCP-HIST-001/003/004/005/006/007; MCP-ERR-001/002/003/004/005; MCP-DOC-001/002/003/006/007/008/009/010; MCP-RES-001/002/003/004.
2. **1 specs are annotated in code but cited by no test** — MCP-TOOL-026.

## Work Required

### Must Fix
- None.

### Should Fix
1. Add `@spec` annotations (entry point + exercising test) for the 67 unlinked specs.
2. Annotate a test for the 1 code-only specs.

### Nice to Have
- Audit each `[x]` against the code and move the segment to `AUDITED`.
