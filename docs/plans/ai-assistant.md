# Plan: TurfOps MCP Server (agentic help over the lawn's data)

Supersedes the earlier in-app chatbot plan (2026-09-22). Decisions agreed with the user:

- **MCP server, not an in-app chat.** The LLM client (Claude Desktop / Code / mobile, via the internal MCP gateway) supplies the agentic loop, history, streaming and UI. TurfOps only exposes tools. The in-app chat is deferred until the MCP version has proven useful (see *Deferred*).
- **Transport:** streamable HTTP, served **in-process** by the existing Axum binary at `/mcp`. No sidecar, no second container, no HTTP self-calls.
- **Scope:** read-only tools first. Write tools (log an application, add a product, …) are phase 2.
- **Auth:** the gateway is open, so `/mcp` requires `Authorization: Bearer <MCP_TOKEN>`. The endpoint is not mounted when the token is unset.

## Why pull (tools) beats push (context injection)

The old plan injected a fixed dashboard snapshot into every prompt. That is token-heavy, always slightly stale, and wrong-shaped for most questions: "how do I kill bentgrass and reseed" needs the fall seeding window, the seed/pre-emergent conflict rule, the shelf and the log — not the disease series. Tools let the model fetch exactly what the question needs, and let one conversation combine TurfOps with the other gateway servers (bronze lake, Home Assistant) without TurfOps knowing about them.

Pull only wins if tools return **answers, not tables**. The REST handlers are already opinionated summaries, so tools mirror them roughly 1:1 with chart series stripped by default, plus one `lawn_snapshot` tool so most questions cost a single round trip.

## Architecture

```
backend/src/mcp/
├── mod.rs          # TurfOpsMcp { state: AppState } + #[tool_router] aggregation, get_info() with instructions, mount()
├── auth.rs         # bearer middleware (constant-time compare), 401 + WWW-Authenticate
├── instructions.rs # the doctrine string (see below)
├── resources.rs    # turfops://docs/* resources (include_str! of docs/)
├── trim.rs         # pure helpers that drop series / cap lists (unit-tested)
└── tools/
    ├── lawn.rs     # lawn_profile, lawn_snapshot, current_conditions, recommendations
    ├── season.rs   # timing_windows, seasonal_plan, gdd, soil_temp_outlook, nitrogen_budget
    ├── disease.rs  # disease_risk, disease_detail
    ├── log.rs      # applications, soil_tests, soil_test_advice
    └── shelf.rs    # products, shopping_list, plants
```

Each `tools/*.rs` is its own `#[tool_router(router = ...)]` impl on `TurfOpsMcp`; `mod.rs` sums them (`+`) into one router. Keeps every file under the 300-line limit.

**Crates:** `rmcp = "3.4"` with features `server`, `macros`, `transport-streamable-http-server`; `schemars` (for `Parameters<T>` input schemas only); `subtle` (constant-time token compare). Verify at implementation time that rmcp's `http`/`tower` majors line up with axum 0.8 / reqwest 0.13 (`cargo tree -d`).

**Mounting:** `StreamableHttpService::new(|| Ok(TurfOpsMcp { state }), Arc<LocalSessionManager>, StreamableHttpServerConfig::default())` nested with `Router::nest_service("/mcp", svc.layer(from_fn_with_state(token, auth::require_bearer)))`. The existing CORS layer stays on `/api` only; `/mcp` is server-to-server. Health (`/api/v1/health`) gains `mcp: "enabled" | "disabled"`.

**Tools call the Axum handlers directly.** Every read handler is a plain `async fn(State<AppState>, Query<P>) -> Result<Json<T>, TurfOpsError>`; the extractors are tuple structs, so a tool is:

```rust
#[tool(name = "gdd", description = "…", annotations(read_only_hint = true))]
async fn gdd(&self, Parameters(p): Parameters<GddParams>) -> Result<CallToolResult, ErrorData> {
    let Json(v) = api::gdd::get_gdd(State(self.state.clone()), Query(GddQuery { year: p.year })).await.map_err(to_mcp)?;
    json_result(&v)   // CallToolResult::success(vec![Content::text(serde_json::to_string(&v)?)])
}
```

No logic is duplicated and no model needs `JsonSchema` (results are JSON text, no output schema). `to_mcp` maps `TurfOpsError` → `ErrorData::internal_error(msg)` with `DataSourceUnavailable` → a message that tells the model the lake/forecast is down so it degrades honestly. Tool parameter structs live in the `mcp` module and derive `Deserialize + JsonSchema`.

## Tools (phase 1, all `read_only_hint = true`)

| Tool | Params | Source | Trim / notes |
|---|---|---|---|
| `lawn_profile` | – | `profile::get_profile` | grass type, zone, soil, size, irrigation |
| `lawn_snapshot` | – | profile + environmental + recommendations + applications + `timing_windows` + `disease_risk` + `nitrogen_budget` via `tokio::join!` | **Call first for any situational question.** Composed rather than wrapping the dashboard, because the dashboard truncates the feed to the top 3 (found in PR 1): full active feed, 5 recent apps, connection status, timing windows without `series`, diseases as `{slug, tier, action, protected_until}` only, N budget totals. Each part degrades to `null` + note on error rather than failing the whole call |
| `current_conditions` | – | `environmental::get_environmental` | full summary + daily forecast; drops the 3-hourly list |
| `recommendations` | `include_inactive?` | `recommendations::list_recommendations` | includes `inventory` status and data points as the UI sees them |
| `timing_windows` | `detail?` | `timing::get_timing_windows` | `series` dropped unless `detail` |
| `seasonal_plan` | – | `seasonal_plan::get_seasonal_plan` | as-is |
| `gdd` | `year?` | `gdd::get_gdd` | as-is |
| `soil_temp_outlook` | – | `soil_temp_prediction` | as-is (crossings + 10 cm outlook) |
| `nitrogen_budget` | – | `nitrogen_budget` | as-is |
| `disease_risk` | – | `disease_risk::get_disease_risk` | per disease: tier, action, factors, management headline, recommended class, on-hand products; `daily` series and methodology dropped |
| `disease_detail` | `slug` | same handler, one disease | everything incl. series, cultural practices, programs, `FungicideOption` list |
| `applications` | `type?`, `since?`, `until?`, `limit?` (default 20, max 100) | `applications::list_applications` / `get_applications_for_profile_in_range` | the log; includes `product_id` linkage and `frac_classes` |
| `soil_tests` | `limit?` | `soil_tests::list_soil_tests` | as-is |
| `soil_test_advice` | – | `soil_tests::get_soil_test_recommendations` | as-is |
| `products` | `category?`, `include_archived?` | `products::list_products` | shelf, with facts + `stock_status`; LLM profile included (informational) |
| `shopping_list` | – | `inventory::get_shopping_list` | as-is |
| `plants` | – | `plants::list_plants` | name, type, location, care-plan windows; full plan JSON is large but bounded |
| `historical` | `range` = 7d/30d/90d | `historical::get_historical` | daily aggregates only |

Descriptions are the doc comments and should tell the model **when** to call the tool (e.g. `timing_windows`: "seeding and pre-emergent windows with this season's state — use for any 'when should I seed / put down pre-em' question; a boundary with `date: null` means later than usual, not unknown").

## Server instructions (the doctrine)

Static string returned in `get_info().instructions`; the profile itself comes from `lawn_profile`. Contents, in this order:

1. What TurfOps is and that it is the **source of truth** for this lawn's state; call `lawn_snapshot` before answering any situational question; don't guess readings.
2. Cool-season lawn (TTTF / KBG / PRG / Mixed) in USDA 7a, SE Pennsylvania; station-local `today` from the lake, which lags about a day; `observed` vs `forecast` must be kept distinct in answers.
3. **No application rates** beyond a product's stored `label_rate`, always with "verify against your label". Never invent products; prefer what is on the shelf; name active ingredient / FRAC class when suggesting chemistry.
4. Seed and pre-emergent never share a season; a logged overseed blocks that season's pre-em and vice versa. Fall pre-em is the alternative to seeding, not a companion.
5. Timing semantics: window states (`OpeningSoon/Open/Ideal/Closing/Closed/Blocked`), `date: null` = later than usual, Tentative vs Observed vs Forecast, "typical" dates are medians of ≤15 years.
6. Disease semantics: only tiers are comparable across diseases (scores are native); `Protected` means a logged fungicide ≥ Good on that disease is inside its residual; red thread's remedy is nitrogen; curative programs are shown for the user's judgment, there is no symptom flag.
7. Recommendation semantics: `inventory` status and `NotOnHand`/`Partial` meaning; answered recs return on escalation or episode end; when an active rec already covers the question, cite its id and reasoning instead of re-deriving.
8. Answer style: reference the specific values that drove the advice; say plainly when data is missing or stale (`soil_observed_at` > 72 h, lake outage notes).

## Resources

- `turfops://docs/timing-windows` → `include_str!("../../../docs/timing-windows.md")` (193 lines; the full window method).
- `turfops://docs/agronomy` → new short `docs/agronomy-methods.md`: the thresholds table from CLAUDE.md, the disease models and tiers, the FRAC efficacy source, the 72 h / 5 d staleness rules. Written once, served verbatim; keep it in sync when thresholds change.

No MCP prompts; the instructions cover it.

## Config

| Env | Meaning |
|---|---|
| `MCP_TOKEN` | Bearer token for `/mcp`. Unset → endpoint not mounted, logged at startup. Rejected at config load if shorter than 32 chars. |

Documented in `backend/.env.example` (placeholder), `docker-compose.yml` (`MCP_TOKEN: ${MCP_TOKEN:-}`), CLAUDE.md env list. The `.env` with the real token is never committed (already gitignored).

## Auth middleware

`auth::require_bearer(State<Arc<str>>, Request, Next)`: parse `Authorization: Bearer …`, compare with `subtle::ConstantTimeEq`, else `401` with `WWW-Authenticate: Bearer realm="turfops-mcp"` and an empty body. No token ever appears in logs or error text. Nothing else on the app changes (the SPA + `/api` stay unauthenticated on the LAN as today).

## Tests

- Unit: auth middleware (missing / wrong / right header → 401 / 401 / 200); `trim.rs` helpers on fixture responses; `list_tools` returns the expected names and every description is non-empty; `get_info` has instructions and `tools` + `resources` capabilities; token length validation in config.
- Handler-backed tools need Postgres and the lake, same as the REST handlers, so they are not unit-tested beyond the pure trimming.
- Manual: `npx @modelcontextprotocol/inspector` against `http://localhost:3000/mcp` with the bearer; then register in the gateway and run the two seed questions in Claude, checking the tool trace:
  - "I'm noticing creeping charlie — what should I do?" → expect `lawn_snapshot` (+ `products`, `applications`), advice keyed to the fall broadleaf timing, whether seeding blocks it, and whether a triclopyr/2,4-D product is on the shelf.
  - "I want to eradicate bentgrass in a section and reseed, now and in the fall" → expect `timing_windows`, `products`, `applications`; advice keyed to the fall seeding window dates and the seed/pre-em rule.

Gates before each push (global rule): `cargo fmt && cargo clippy && cargo test`; frontend unchanged.

## PR plan (branch + PR each, per project rule)

1. **`feat(mcp): server skeleton, bearer auth, core tools`** — rmcp dep, `mcp/` module, `MCP_TOKEN`, `/mcp` mount, health flag, instructions, `lawn_profile`, `lawn_snapshot`, `current_conditions`, `recommendations`, `applications`. Usable end-to-end on its own.
2. **`feat(mcp): season, disease, log and shelf tools + resources`** — the remaining tools, `trim.rs`, both resources, `docs/agronomy-methods.md`, CLAUDE.md + README updates.

## Deferred (not in scope)

- **Write tools** (`log_application`, `add_product`, `set_stock_status`, `answer_recommendation`, `record_soil_test`): same bearer; `destructive_hint`/`idempotent_hint` annotations; reuse the POST/PATCH handlers the same way. Decide after 2 weeks of read-only use.
- **In-app chat.** If wanted later it is a thin backend tool-use loop over the *same* `tool_router` against OpenRouter, plus a page. The tool registry is the reusable part; never build the mega-prompt version.
- **Open-Meteo, quantities, rates** — unchanged policy.

## Kill criteria (carried over from the old plan)

Use it for two weeks. Keep it if most questions lean on the tool results rather than general knowledge, and if it has at least once surfaced something the dashboard alone would not have. If you are fact-checking its product suggestions elsewhere, tighten the instructions or strip product naming before adding write tools. Rip-out cost is one module, one dep and one env var.
