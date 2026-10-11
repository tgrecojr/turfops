---
parent: high-level-design
prefix: MCP
---

# MCP Server

## Context and Design Philosophy

The MCP server gives an AI client acting for the owner read access to the lawn. The client is Claude, reached through the owner's internal MCP gateway. It exposes the same answers the web UI shows as Model Context Protocol tools, plus the two methodology documents as resources. It is built into the backend binary (`backend/src/mcp/`) and served at `/mcp` over streamable HTTP.

Guiding principles:

- **One answer everywhere.** A tool calls the REST handler that backs the matching page and returns that handler's body, trimmed. No agronomic logic lives in `mcp/`. The one exception is `weather_history`, which composes two existing lake reads because no handler gives a true daily series (see *weather_history*).
- **Pull, not push.** The client supplies the agent loop, history and UI. TurfOps supplies tools that return answers, not tables. `lawn_snapshot` answers most situational questions in one round trip, and narrower tools give the detail.
- **Read-only.** Every tool is annotated `read_only_hint = true`, and no tool reaches a write handler. Agents read and the owner changes state through the UI (HLD non-goal: *writes through MCP*).
- **Withhold rather than invent, at the protocol boundary too.** A tool that fails returns a model-readable error that tells the model to report the gap, not estimate around it. A composite tool that loses one part returns `null` for that part with a note.
- **The bearer token is the guard.** The gateway in front of TurfOps is open, so `/mcp` is unreachable without `Authorization: Bearer <MCP_TOKEN>`, and the endpoint does not exist at all when no token is configured.
- **One clock.** "Today" and every date the tools compute is the lawn's local date (the app's `Clock`, resolved from `TZ`, which deployment sets to the lawn's timezone), the same clock every other component uses (HLD decision *"Today" and every date gate*). Handler-backed tools inherit the handler's dates; the only date this component computes itself is `weather_history`'s window start.

## Module Layout

| File | Owns |
|---|---|
| `backend/src/mcp/mod.rs` | `TurfOpsMcp { state: AppState, tool_router }`, `tools()` (sum of the per-area routers), the `ServerHandler` impl (`get_info`, `list_resources`, `read_resource`), `server_config()`, `router(state, token)` |
| `backend/src/mcp/config.rs` | `McpConfig { token }`, `parse_token`, `MIN_MCP_TOKEN_LEN = 32`, redacting `Debug` |
| `backend/src/mcp/auth.rs` | `require_bearer` middleware, `authorized` |
| `backend/src/mcp/instructions.rs` | `INSTRUCTIONS`, the doctrine string returned at `initialize` |
| `backend/src/mcp/resources.rs` | `DOCS` (two embedded markdown docs), `list`, `read` |
| `backend/src/mcp/trim.rs` | Pure `serde_json::Value` reshaping helpers and composite-part helpers |
| `backend/src/mcp/tools/mod.rs` | `call`, `call_trimmed`, `to_value`, `json_result`, `tool_error`, `error_message` |
| `backend/src/mcp/tools/{lawn,season,weather,disease,log,shelf}.rs` | One `#[tool_router(router = <area>_tools)]` impl each, plus that area's parameter structs |

Each `tools/*.rs` is its own router so that every file stays under the 300-line limit. `TurfOpsMcp::tools()` combines them with `+` (`mod.rs:35-42`).

## Configuration and Mounting

**`MCP_TOKEN`** (`config.rs:25-32`, read in `backend/src/config.rs:282-284`):

| Raw value | Result |
|---|---|
| unset, empty or whitespace only | `None`: `/mcp` is not mounted |
| fewer than 32 characters after trimming | `TurfOpsError::Config`: startup fails |
| 32 or more characters | `Some(trimmed token)` |

An empty value means "not configured" because Compose passes unset keys as `""` (`docker-compose.yml`: `MCP_TOKEN: ${MCP_TOKEN:-}`). A token that is set but short fails startup instead of leaving a weak endpoint. `McpConfig`'s `Debug` prints `[REDACTED]`. This component owns the `MCP_TOKEN` rules, the mounting and the exclusion from CORS (MCP-CFG-001..005); Deployment and the web UI only pass the variable through and consume the route's absence.

**Mounting** (`backend/src/main.rs:206-216`). The API router is built first with its 1 MiB body limit, its JSON error handling (Deployment) and CORS layer. Only then is `mcp::router(state, &token)` merged in, so the API's layers do not apply to `/mcp`: the route is server-to-server and has no browser origin. `/mcp` carries its own 1 MiB request-body limit, the same as `/api/v1`, and a larger body is rejected with `413` before any tool runs (MCP-SRV-006). **Divergence:** the `/mcp` router sets no limit of its own, so the only cap is rmcp's default `max_request_body_bytes` of 4 MiB (Deferred 4). With no token, startup logs `MCP_TOKEN not set — /mcp not mounted` and `/mcp` falls through to the SPA static fallback like any unknown path. `AppState.mcp_enabled` records whether the route is mounted, and `GET /api/v1/health` reports it as `mcp: "enabled" | "disabled"` (`backend/src/api/health.rs:14-40`).

**Transport** (`mod.rs:85-108`). `StreamableHttpService` (rmcp 3.x, locked at 3.5.1) is nested at `/mcp` with this config:

| Setting | Value | Why |
|---|---|---|
| `with_legacy_session_mode(false)` | stateless | Every tool is a single request and response, so there is no session to keep, and a server restart never strands a client |
| `with_json_response(true)` | plain JSON bodies | No streaming is needed |
| `with_sse_keep_alive(None)` | off | No SSE streams |
| `disable_allowed_hosts()` | Host check off | The gateway calls in by LAN hostname, so rmcp's loopback-only default would reject it. The check exists to stop DNS rebinding, and a rebinding page cannot send the bearer token |

**Methods.** Stateless streamable HTTP with JSON responses needs only `POST`: there is no session to open a `GET` stream on or to `DELETE`. A request that passes the bearer guard with any other method gets `405 Method Not Allowed` with `Allow: POST` (MCP-SRV-007). An unauthenticated request of any method gets the guard's `401` first. **Divergence:** TurfOps does not route by method; the service is mounted with `nest_service` (`mod.rs:103`), and the `405` comes from rmcp 3.5.1's own stateless dispatch, which nothing in TurfOps pins (Deferred 13).

A fresh `TurfOpsMcp` is built for each request from a cloned `AppState` and the prebuilt tool router. The `LocalSessionManager` argument is required by the constructor but goes unused in stateless mode.

**Server info** (`mod.rs:68-77`): capabilities `tools` and `resources`; server name `turfops`; version `CARGO_PKG_VERSION`; and `instructions` set to `INSTRUCTIONS`. The server offers no prompts.

## Authentication

`auth::require_bearer` is an Axum `from_fn_with_state` layer on the `/mcp` router, so it runs before rmcp sees the request (`auth.rs:14-40`).

- It accepts only an `Authorization` header that is valid ASCII and splits at the first space into a scheme and a value. The scheme must match `bearer` case-insensitively (RFC 9110). The value is trimmed and compared with `subtle::ConstantTimeEq`.
- Anything else gets `401` with `WWW-Authenticate: Bearer realm="turfops-mcp"` and an empty body. That covers a missing header, a wrong token, a prefix of the token, another scheme or a non-ASCII header.
- The token never appears in a log line or a response.
- Nothing else on the app is authenticated. The SPA and `/api` stay open on the LAN.

## Tool Catalog

Every tool is `read_only_hint = true`. Its description is written to tell the model **when** to call it. A tool returns one JSON text content block, made by `json_result`, which uses `ContentBlock::json`. Parameter structs derive `Deserialize + JsonSchema`, and every parameter is optional.

| Tool | Parameters | Calls | Shaping |
|---|---|---|---|
| `lawn_profile` | none | `api::profile::get_profile` | as-is |
| `lawn_snapshot` | none | seven handlers concurrently + Data sync's cached connection status | see *lawn_snapshot* |
| `current_conditions` | none | `api::environmental::get_environmental` | `forecast.hourly` dropped; daily summaries kept |
| `recommendations` | `include_inactive` (bool, default false) | `api::recommendations::list_recommendations` | as-is, in the feed's order (inventory status, data points, answered flags) |
| `timing_windows` | `detail` (bool, default false) | `api::timing::get_timing_windows` | `series` dropped unless `detail` |
| `seasonal_plan` | `year` (i32) | `api::seasonal_plan::get_seasonal_plan` | as-is |
| `gdd` | `year` (i32) | `api::gdd::get_gdd` | `daily_history` dropped (the description points to `weather_history`) |
| `soil_temp_outlook` | none | `api::soil_temp_prediction::get_soil_temp_forecast` | as-is (10 cm outlook + threshold crossings) |
| `nitrogen_budget` | `year` (i32) | `api::nitrogen_budget::get_nitrogen_budget` | as-is (includes the per-application list) |
| `weather_history` | `days` (i64, default 30, clamped 1–120) | lake client directly (see *weather_history*) | own row shape |
| `disease_risk` | none | `api::disease_risk::get_disease_risk` | `trim::disease_overview` |
| `disease_detail` | `slug` (string, required) | `api::disease_risk::get_disease_risk` | `trim::disease_detail` for one disease |
| `applications` | `type`, `since`, `until` (YYYY-MM-DD, inclusive), `limit` (default 20, clamped 1–100) | `api::applications::list_applications`, or `queries::get_applications_for_profile_in_range` when a date bound is given | as-is (`product_id`, `frac_classes` included) |
| `soil_tests` | `limit` (default 5, clamped 1–50) | `api::soil_tests::list_soil_tests` | as-is |
| `soil_test_advice` | none | `api::soil_tests::get_soil_test_recommendations` | as-is |
| `products` | `category` (case-insensitive enum name), `include_archived` (bool) | `api::products::list_products` | as-is; the description marks `profile` as AI background, not fact |
| `shopping_list` | none | `api::inventory::get_shopping_list` | as-is |
| `plants` | none | `api::plants::list_plants` | as-is (full care plans) |

**The feed in the tools.** `recommendations` and `lawn_snapshot` relay the recommendation feed exactly as the feed returns it: sorted by descending severity, ties in source order, the same order as the dashboard and the Recommendations page. Neither tool reorders, filters or re-scores it. The feed carries no soil-test advice, so neither tool does; `soil_test_advice` is the soil-test surface. The doctrine's "cite an active rec" therefore always points at advice the owner also sees in the UI, in the same order.

**Trimming.** `trim.rs` reshapes `serde_json::Value`s by field name (`hourly`, `series`, `daily_history`, `applications`, and the disease picks such as `management.preventative.options[].recommended` and `protection.protected_through`). A renamed field in a handler model does not fail compilation; it silently leaves bulk in the reply or turns a picked field `null`. Each trim is therefore tested against a body serialized from the handler's own response model, so a rename fails a test (MCP-TOOL-027). **Divergence:** the trim tests use hand-written JSON fixtures (Deferred 3).

The 18 names are pinned by a test (`mod.rs:135-167`). Another test asserts that every tool has a non-empty description and is read-only (`mod.rs:115-133`).

### Parameter handling owned by this component

- **`applications`** (`tools/log.rs:54-81, 156-184`). With no `since`/`until`, it delegates to the list handler, which validates `type` and orders newest first. With either bound it builds a half-open `[since, until+1)` range. An open side is bounded by `1900-01-01` or `9999-12-31`. A bad date or `since > until` is `InvalidData`. The type filter and `limit` are applied in the database query, so a wide range never loads more than `limit` rows (MCP-TOOL-026). An unknown `type` is reported with the list handler's wording, `Unknown application type filter: X`, on both paths, so the model sees one message for one mistake (MCP-TOOL-018). **Divergence:** the date path validates `type` with its own wording, `Unknown application type: X` (`tools/log.rs:129`), fetches the whole range, then filters by type and takes `limit` in Rust (`tools/log.rs:139-150`) (Deferred 5).
- **`products`** (`tools/shelf.rs:42-51`). An unparseable `category` is `InvalidData("Unknown product category `X`. Use one of: …")`, listing every category by enum name, as `disease_detail` lists its slugs, so the model can correct the call without guessing (MCP-TOOL-022). **Divergence:** the message omits the list (`tools/shelf.rs:47`); only the parameter's doc comment names the categories (Deferred 6).
- **`disease_detail`** (`tools/disease.rs:62-73`). The slug is trimmed, lowercased, and spaces and underscores become hyphens. An unknown slug is `InvalidData` listing the slugs actually present in the response.

## Handler Bridge and Error Mapping

Tools call handlers as plain async functions with hand-built extractors, for example `State(self.state.clone())` and `Query(GddQuery { year })`. There are no HTTP self-calls. Results carry no output schema, so no response model needs `JsonSchema`.

| Helper (`tools/mod.rs`) | Behaviour |
|---|---|
| `call(handler)` | `Ok(Json(body))` → `json_result(&body)`; `Err(e)` → `Ok(tool_error(e))` |
| `call_trimmed(handler, trim)` | The same, but the body goes through `serde_json::to_value`, then `trim`, then serialization |
| `to_value(handler)` | The handler's body as a `Value`, for trimming or composition |
| `tool_error(e)` | `CallToolResult::error` (`isError: true`) with one text block from `error_message(e)` |

Handler failures are therefore **tool-level** errors that the model can read and relay, never JSON-RPC protocol errors. A result body that fails to serialize is treated the same way: a tool error with the internal-error text, the cause logged as an error (MCP-ERR-007). The only protocol-level `ErrorData` is for an unknown resource URI. **Divergence:** `json_result` propagates `ContentBlock::json`'s `ErrorData` (`tools/mod.rs:19-21`), so a serialization failure reaches the client as a JSON-RPC error (Deferred 11).

`error_message` (`tools/mod.rs:31-57`) decides what the model may see:

| `TurfOpsError` | Text returned to the model | Logged |
|---|---|---|
| `NotFound(msg)`, `InvalidData(msg)` | `msg` verbatim (messages written for users) | no |
| `Unavailable { origin, outage }` | `Data source unavailable: {owner sentence} Tell the user this data is missing rather than estimating it.` The owner sentence is `owner_message()`: which source is unavailable, what it affects and what to do. No upstream body, URL or task error | `warn`, with the detail |
| `DataLake(_)` | Fixed text: the NOAA lake could not be read, so station soil, timing windows and disease risk are unavailable, plus the same hint. No path or SQL | `warn` |
| `Http(_)` | Fixed text: an external source (forecast or Home Assistant) did not respond, plus the hint | `warn` |
| anything else (`Database`, `Config`, `Internal`, `Io`, `Json`) | `TurfOps hit an internal error answering this; details are in the server log.` | `error` |

The REST layer shows the user the same `NotFound` and `InvalidData` messages and the same outage sentence (`owner_message()`, `backend/src/error.rs`). The MCP mapping therefore exposes nothing the UI does not already show, and it is stricter for lake and HTTP errors.

## lawn_snapshot

`lawn_snapshot` (`tools/lawn.rs:50-113`) is the tool the doctrine tells the model to call first. It is composed from handlers rather than wrapping `/api/v1/dashboard`, because the dashboard truncates the feed to three items.

**Parts**, fetched with `tokio::join!`:

| Key | Handler | Shaping |
|---|---|---|
| `profile` | `profile::get_profile` | as-is |
| `conditions` | `environmental::get_environmental` | `forecast.hourly` dropped |
| `recommendations` | `recommendations::list_recommendations` (`include_inactive: false`) | the **full** active feed, in the feed's severity order |
| `recent_applications` | `applications::list_applications` (`limit: 5`, no type) | the 5 latest log entries |
| `timing_windows` | `timing::get_timing_windows` | `series` dropped |
| `disease_risk` | `disease_risk::get_disease_risk` | `trim::disease_headlines`: per disease `slug, name, tier, tier_note, score_label, as_of, action, headline, protected_through`, plus top-level `today` and `data_notes` |
| `nitrogen_budget` | `nitrogen_budget::get_nitrogen_budget` (current year) | `trim::nitrogen_totals` (per-application list dropped) |

After the join, `data_sources` is set from Data sync's connection status, under Data sync's key names (`soildata` for the lake). The snapshot reads Data sync's **cached** probe results, which Data sync refreshes per source once a result is older than 5 minutes (MCP-SNAP-009). The snapshot is the tool the model calls first, often several times in a conversation, so live probes would add an OpenWeatherMap request and a Home Assistant ping, with their latency and OWM quota, to every call. **Divergence:** the tool calls `DataSyncService::check_connections()` under the sync service's read lock (`tools/lawn.rs:76-82`), which probes every source live on each call (Deferred 2).

**Degradation.** Each part goes through `trim::part`. A failure becomes `null` at its key, and `notes` gains `"<part> unavailable: <error_message text>"`. A lake outage, for example, nulls `timing_windows` and `disease_risk` but leaves the feed, log and profile intact. The snapshot as a whole always succeeds: `notes` is always present (possibly empty). `data_sources` degrades like every other part: if the connection status cannot be serialized, it is `null` and `notes` gains `"data_sources unavailable: <internal-error text>"` (MCP-SNAP-010). **Divergence:** `serde_json::to_value(connections).unwrap_or_default()` (`tools/lawn.rs:110`) leaves `null` with no note (Deferred 12).

The concurrent handlers share the Data sync refresh, the climate-record memo and the database pool. Contention between them is resolved inside those components, not here.

## weather_history

`weather_history` (`tools/weather.rs`) returns one row per local calendar day for the last `days` days. It does **not** call `/api/v1/historical`. That handler downsamples hourly points for charts, so summing its rain per day gives wrong totals. Instead the tool joins two daily lake reads from the Weather data access component:

1. `WeatherLakeClient::fetch_daily_disease_inputs(start)`: silver hourly data aggregated per `obs_date_local`. It supplies air mean/min/max (°C), mean RH, rain (mm) and `hours_covered`.
2. `api::timing::climate_record(state, client)`: the 15-year climate record memoized for 1 hour, owned by Timing windows. It supplies the daily 5 cm soil mean (°F), gold air min and mean (°F) and gold `gdd50`.

`start` is the lawn's local today minus `days`: `history_start(self.state.clock.today(), days)` (`tools/weather.rs`), so the window holds `days` local days whatever the hour. The rows themselves are keyed by the silver layer's `obs_date_local`, the station's civil date in America/New_York, which is the same calendar `TZ` gives (Weather data access). The two reads run with `tokio::try_join!`, so either failing fails the tool through `error_message`. If no lake client is configured, the tool returns `TurfOpsError::not_configured(Source::WeatherLake)`, which reaches the model as the lake-not-configured sentence.

**Join** (`join_days`, `tools/weather.rs:105-138`, a pure function). Both inputs are filtered to `date >= start` and merged into a `BTreeMap` keyed by date, ascending. A date present in either source gets a row.

| Field | From | Unit / rounding |
|---|---|---|
| `air_mean_f`, `air_min_f` | silver (°C → °F); gold fills them when silver has no value | °F, 0.1 |
| `air_max_f` | silver only | °F, 0.1 |
| `rh_mean_pct` | silver | %, 0.1 |
| `rain_in` | silver `precip_mm / 25.4` | in, 0.01 |
| `hours_observed` | silver `hours_covered` | count |
| `soil_5cm_f`, `gdd50` | climate record | 0.1 |

Totals: `rain_total_in` sums the days that have `rain_in`, and `gdd50_total` sums the days that have `gdd50`. A fixed `note` explains that a day with fewer than 24 hours observed is partial. The newest day is usually partial because the lake lags about a day.

## Doctrine and Resources

**Instructions** (`instructions.rs`) form a static string. The lawn's particulars come from `lawn_profile`, not the doctrine. In order, it covers:

1. **Source of truth.** Call `lawn_snapshot` first, then use narrower tools. Never guess a reading, date or product. Read the resources for "why" questions.
2. **Setting.** A cool-season lawn in zone 7a, southeast Pennsylvania. Lake-lagged data; `today` is the lawn's local date; observed values kept distinct from forecast values; °F; timing uses 5 cm soil, while the live reading and the grub window use 10 cm. The text tells the model that `today` "is the lawn's local date (the server's `TZ`)" (`instructions.rs`) and names no second clock (MCP-DOC-004; asserted in `mod.rs` tests).
3. **Products and rates.** Never state a product's label rate, and never any rate for a pesticide (fungicide, herbicide, insect control); the label governs. Agronomic amounts are part of the advice and may be relayed: nutrient per 1,000 sq ft (the nitrogen budget is in lb N/1,000 sq ft), lime or sulfur from a soil test, seed per 1,000 sq ft, mulch depth. A nutrient amount may be converted into an amount of a fertilizer or soil-amendment material, but not of a material that is also a pesticide active (give the nutrient amount only). When the owner asks about a specific shelf product, its stored `label_rate` may be reported as the owner's recorded value with "verify against your label"; it is never offered unprompted or used inside advice. Never invent products, and prefer the shelf. Off the shelf, name the active ingredient and, for fungicides, the FRAC class, not a brand. **Divergence:** `instructions.rs:24-25` says "Never give an application rate except a product's stored `label_rate`, and then always add 'verify against your label'". That forbids the agronomic amounts the feed itself carries and lets the model quote the label rate in any answer, not only when asked about that product (Deferred 10).
4. **Seed vs pre-emergent.** The two never share a season, and the state is `Blocked`. Fall pre-emergent is the alternative to fall seeding, and aeration belongs to the seeding window.
5. **Timing windows.** The eight window states and the four date sources. `date: null` means later than usual, never unknown.
6. **Disease risk.** Only tiers are comparable, and `Protected` has a fixed meaning. Red thread is treated with nitrogen. TurfOps has no symptom input, so the model asks the owner before recommending a curative. Rotate classes, and call `disease_detail` before naming a fungicide.
7. **Recommendations.** Cite an active rec instead of re-deriving it. The meanings of `inventory.state`. Answered recs return on escalation or when the episode ends.
8. **Answer style.** Name the driving values, and state staleness plainly: `soil_observed_at` older than 72 h, `data_notes`, or an unavailable source.

A test pins the presence of `lawn_snapshot` and `verify against your label` (`mod.rs:179-182`); the second assertion encodes the divergent rates wording and changes with it. The doctrine's vocabulary (window states, date sources, `inventory.state`) mirrors enums owned by Timing windows and Product inventory. Changing those enums requires changing this text.

**Resources** (`resources.rs`). Both are `text/markdown`, embedded with `include_str!` at build time, and listed with their byte size:

| URI | Name | Source |
|---|---|---|
| `turfops://docs/agronomy` | `agronomy-methods` | `docs/agronomy-methods.md` |
| `turfops://docs/timing-windows` | `timing-windows` | `docs/timing-windows.md` |

An unknown URI returns `ErrorData::resource_not_found`. `docs/agronomy-methods.md` exists to be served here and must be kept in sync with the thresholds in code.

**Build coupling** (`Dockerfile:31-33`). The Rust build stage copies only `backend/src` to `/app/src`, and from `/app/src/mcp/` the path `../../../docs` resolves to `/docs`. The Dockerfile therefore copies the two documents to `/docs/` before `cargo build`. A missing document is a compile error, not a runtime one.

## Decisions & Alternatives

| Decision | Chosen | Alternatives Considered | Rationale |
|---|---|---|---|
| Form of AI access | MCP server; the client supplies the loop and UI | In-app chat with a fixed dashboard snapshot injected into the prompt | A fixed snapshot is token-heavy, stale and the wrong shape for most questions. Tools fetch what the question needs and combine with the owner's other gateway servers (bronze lake, Home Assistant). |
| Hosting | In-process at `/mcp`, streamable HTTP, rmcp | Sidecar container; stdio server; HTTP self-calls to `/api` | Plan decision: no second container and no self-calls, and the tools reuse `AppState` directly. |
| Scope | Read-only tools only | Write tools (log application, add product, set stock, answer rec, record soil test) | Owner decision: read-only first; write tools to be decided after about two weeks of use. Read-only also limits the damage a wrong call can do. |
| Auth | Bearer token from `MCP_TOKEN`, constant-time compare, ≥32 chars, not mounted when unset | Network trust only; OAuth | Owner decision: the gateway is open, so the endpoint needs its own guard. A short token fails startup rather than mounting a weak endpoint. |
| Tool implementation | Call the Axum handlers as functions | Re-implement queries in `mcp/`; HTTP to `/api` | Plan decision: no duplicated logic, and the answers match the UI exactly (HLD tenet *one answer everywhere*). |
| Result format | JSON text content, no output schema | Typed output schemas | Plan decision: no response model has to derive `JsonSchema`. |
| Session mode | Stateless, JSON responses, no SSE | Stateful sessions with SSE | Every tool is one request and one response, and a restart never strands a client. [inferred] |
| Host header check | Disabled | rmcp loopback-only default; allow-listing the gateway host | The gateway calls by LAN hostname. The check guards against DNS rebinding, which cannot forge the bearer token. |
| CORS | `/mcp` merged after the API's CORS layer | Sharing the API's CORS layer | Plan decision: `/mcp` is server-to-server, so CORS does not apply. |
| Request body limit on `/mcp` | 1 MiB, the same as `/api/v1` | rmcp's 4 MiB default; no limit | Tool calls carry a few small parameters, so 1 MiB is ample. One limit across the app's two entry points leaves nothing to reason about separately, and a behind-the-bearer endpoint still should not buffer arbitrarily large bodies. |
| HTTP methods on `/mcp` | `POST` only; anything else `405` with `Allow: POST` | Pass every method to rmcp and rely on its handling | Stateless JSON mode has no `GET` stream and no session to `DELETE`. Stating the method set makes the surface explicit instead of an rmcp implementation detail. |
| Snapshot composition | Composed from seven handlers | Wrapping `/dashboard` | The dashboard truncates the feed to three items, and the snapshot must carry the full feed. |
| Feed order in tools | Relay the feed's own order (descending severity, ties in source order) unchanged | Sorting in the tool; source order | One answer everywhere: the model sees the most severe advice first, and its first three items are the dashboard's three alerts. Ordering lives in the feed, so no surface sorts on its own. |
| Partial failure in the snapshot | `null` part + `notes` entry, `data_sources` included | Failing the whole call; a silent `null` for `data_sources` | Plan decision: a lake outage must not hide the feed, the log or the profile. Every part, the connection status included, explains its absence, so the model never mistakes a missing part for an empty one. |
| Snapshot connection status | Data sync's cached probe results (re-probed per source after 5 minutes) | Live probes on every call | The snapshot is called first and often. Live probes cost an OWM request and an HA ping per call, and a status up to 5 minutes old answers "is a source down" as well as a live one. |
| Handler errors | `isError` tool result with model-readable text | JSON-RPC `internal_error` (`ErrorData`) | A tool-level error is shown to the model, which can then tell the user what is missing. A protocol error tends to be treated as a transport failure. Internal details stay in the log. [inferred] |
| Reply serialization failure | `isError` tool result with the internal-error text | JSON-RPC error from `ContentBlock::json` | It is an internal failure like any other, so it gets the same tool-level treatment and the same log entry; the client never sees a transport-style error for a tool call. |
| Unknown-value errors | Name the bad value and list the valid ones (`disease_detail` slugs, `products` categories); one wording per mistake on every path (`applications` type) | Bare "unknown X"; per-path wording | The model can correct its own call from the error alone, and the same mistake reads the same however the tool was called. |
| Daily weather | `weather_history` joining silver per-day aggregates with the climate record | Wrapping `/historical` | `/historical` downsamples hourly points for charts, so a daily rain sum from it is wrong. |
| Disease trimming | Overview keeps the assessment and the recommended preventative; `disease_detail` has everything | Returning the full response every time | Plan decision: the daily series, methodology and full programs are bulky. The detail tool exists for "what should I spray". |
| Guarding the trims | Test each trim against a body serialized from the handler's response model | Typed trims over the models; hand-written JSON fixtures | Trimming by field name keeps `mcp/` free of response-model types, and a serialized-model test catches a rename that a fixture would hide. |
| Router layout | One `#[tool_router]` per area, summed | One impl with 18 tools | Keeps each file under the 300-line limit. |
| Methodology access | Two `include_str!` resources + doctrine; no MCP prompts | Prompts; fetching the docs at runtime | Plan decision: the docs ship with the binary and cannot drift from the build. The doctrine covers what prompts would. |
| Model's notion of today | The lawn's local date (the app's `Clock`, `TZ` = the lawn's timezone), inherited from the handlers and used for `weather_history`'s window start | The UTC date; a station-local date from the lake's UTC offset | Every agronomic date is a lawn-local calendar day, and one configured clock gives every surface, the MCP tools included, the same day. |
| Rates in agent answers | Agronomic amounts relayed (and convertible to a fertilizer or amendment material); never a pesticide rate; a stored label rate only when the owner asks about that product, with "verify against your label" | No amounts at all; quoting the stored label rate in any answer | Nutrient, lime and seed amounts are agronomy and do not depend on the product. How much of a product to apply is set by its label, the legal authority, above all for pesticides; a recorded label rate is the owner's own data, so reporting it on request answers a question rather than giving advice. The doctrine text diverges (Deferred 10). |

## Open Questions & Future Decisions

### Resolved
1. ✅ Read-only first; write tools deferred until the read-only version has proven useful (owner).
2. ✅ No in-app chat. If one is ever wanted, it is a tool-use loop over this same router, never a prompt-injection design (owner).
3. ✅ `/mcp` requires a bearer token because the gateway is open, and is not mounted when the token is unset (owner).
4. ✅ `weather_history` replaces a planned `historical` tool, for the daily-rain reason above.

### Deferred
1. **Write tools.** `log_application`, `add_product`, `set_stock_status`, `answer_recommendation`, `record_soil_test`: same bearer, `destructive_hint`/`idempotent_hint` annotations, reusing the POST/PATCH handlers. The kill criteria in `docs/plans/ai-assistant.md` apply first.
2. **Divergence: live connection probes in `lawn_snapshot`** (MCP-SNAP-009). Intent: `data_sources` comes from Data sync's cached probe results, re-probed per source after 5 minutes. The tool calls `check_connections` on every `lawn_snapshot`, after the join and under the sync service's read lock (`tools/lawn.rs:76-82`): a live OWM `/weather` call plus a Home Assistant ping each time. The probe cache itself belongs to Data sync.
3. **Divergence: trims tested only against hand-written fixtures** (MCP-TOOL-027). Intent: each trim is tested against a body serialized from its handler's response model. `trim.rs` matches keys such as `series`, `hourly`, `applications`, `management.preventative.options[].recommended` and `protection.protected_through` (`trim.rs:9, 25, 30, 94, 107`), and its tests build those bodies by hand, so renaming a field in a handler model silently stops the trim.
4. **Divergence: `/mcp` body limit** (MCP-SRV-006). Intent: 1 MiB, rejected with `413`, as on `/api/v1`. The 1 MiB `RequestBodyLimitLayer` (`main.rs:199`) covers `/api` only, because `/mcp` is merged afterwards (`main.rs:210`), and `server_config()` leaves rmcp's `max_request_body_bytes` at its 4 MiB default.
5. **Divergence: `applications` date path** (MCP-TOOL-018, MCP-TOOL-026). Intent: one wording, `Unknown application type filter: X`, on both paths, and the type filter and `limit` applied in the database query. Without dates the handler reports `Unknown application type filter: X` (`api/applications.rs:49`); with dates the tool reports `Unknown application type: X` (`tools/log.rs:129`) and fetches the whole range before filtering and applying `limit` in Rust (`tools/log.rs:139-150`).
6. **Divergence: `products` category error** (MCP-TOOL-022). Intent: the error lists every valid category, as `disease_detail`'s unknown-slug error lists the slugs. The message at `tools/shelf.rs:47` names only the bad value; the parameter's doc comment is the only place the categories appear.
7. **Function and file length.** `lawn_snapshot` (`tools/lawn.rs:50-113`) is about 63 lines, over the project's 50-line limit. `trim.rs` is 295 lines, close to the 300-line limit.
8. **Tool-call coverage.** Handler-backed tools need PostgreSQL and the lake, so tests cover only the pure parts: trimming, the date range, `join_days`, auth, config, registration and resources. No test exercises a tool end to end.
9. **Rain undercount boundary.** `weather_history` rain comes from silver hours with non-null air temperature and `rh_pct > 0` (`datasources/weather/disease.rs:48`, Weather data access). Rain in hours when those sensors dropped out is not counted. Whether that is the intended contract belongs to Weather data access.
10. **Divergence: the doctrine's rates rule** (`instructions.rs:24-25`). "Never give an application rate except a product's stored `label_rate`" both forbids agronomic amounts the tools return (lb N/1,000 sq ft and the remaining nitrogen budget in the feed, lime or sulfur in `soil_test_advice`) and lets the model quote a stored label rate in any answer. The intended rule: relay agronomic amounts, converting to a fertilizer or amendment material when useful but never to a material that is also a pesticide active; never state a pesticide rate; report a stored label rate only when the owner asks about that product, with "verify against your label". The test assertion on `verify against your label` (`mod.rs:181`) stays with the on-request wording.
11. **Divergence: serialization failure is a protocol error** (MCP-ERR-007). Intent: a tool error with the internal-error text, logged as an error. `json_result` propagates `ContentBlock::json`'s `ErrorData` (`tools/mod.rs:19-21`), so a body that fails to serialize reaches the client as a JSON-RPC error. No current model is known to fail serialization.
12. **Divergence: `data_sources` can be `null` silently** (MCP-SNAP-010). Intent: a `null` `data_sources` comes with a `notes` entry, like every other part. The connection status is serialized with `unwrap_or_default` (`tools/lawn.rs:110`), so a failure leaves `null` with no note.
13. **Divergence: `/mcp` method set is not pinned** (MCP-SRV-007). Intent: only `POST`; any other method that passes the bearer guard gets `405` with `Allow: POST`. The service is mounted with `nest_service` (`mod.rs:103`), so every method reaches rmcp. rmcp 3.5.1 in stateless mode without an event store answers non-`POST` with exactly that `405`, but no TurfOps route or test holds it, so an rmcp upgrade or a config change could alter it unnoticed.

## References

- `docs/high-level-design.md`: *Agent access through MCP*; Key Design Decisions row *AI access through a read-only MCP server*.
- `docs/plans/ai-assistant.md`: the agreed plan (tools, doctrine, auth, kill criteria).
- `docs/agronomy-methods.md`, `docs/timing-windows.md`: served as resources.
- Boundary components: Recommendation feed (`recommendations`, snapshot feed), Timing windows (`timing_windows`, `climate_record`), Disease risk (`disease_risk`, `disease_detail`), Weather data access (`fetch_daily_disease_inputs`, the lake client), Data sync (`get_environmental`, `check_connections`), Applications, Product inventory (`products`, `shopping_list`), Soil tests, Landscape plants, Seasonal plan, Agronomic rules (`gdd`, `nitrogen_budget`), Deployment (Dockerfile docs copy, Compose `MCP_TOKEN`).
- Model Context Protocol, streamable HTTP transport; rmcp crate (Rust MCP SDK).
