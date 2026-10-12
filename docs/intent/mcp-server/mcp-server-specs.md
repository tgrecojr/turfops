# MCP Server — EARS Specs

Design: [mcp-server-design.md](mcp-server-design.md). Prefix `MCP`. Facets: `CFG` (`MCP_TOKEN` and mounting), `AUTH` (bearer guard), `SRV` (transport, server info, read-only contract), `TOOL` (tool catalog, handler mapping, parameters and trimming), `SNAP` (`lawn_snapshot`), `HIST` (`weather_history`), `ERR` (handler errors as tool errors), `DOC` (the doctrine returned at `initialize`), `RES` (methodology resources).

A "tool error" below is an MCP tool result with `isError: true` and one text block, which the model reads and relays; it is not a JSON-RPC protocol error. "The missing-data hint" is the sentence "Tell the user this data is missing rather than estimating it."

## Configuration and mounting

- [x] **MCP-CFG-001**: When `MCP_TOKEN` is unset, empty or whitespace only, the system shall not mount `/mcp`, shall log "MCP_TOKEN not set — /mcp not mounted", and `/mcp` shall fall through to the SPA static fallback like any unknown path.
- [x] **MCP-CFG-002**: If `MCP_TOKEN`, trimmed, is non-empty and shorter than 32 characters, then the system shall fail startup with a configuration error saying the MCP token must be at least 32 characters, rather than mount a weakly guarded `/mcp`.
- [x] **MCP-CFG-003**: When `MCP_TOKEN`, trimmed, is 32 characters or longer, the system shall mount `/mcp` and require that trimmed token as its bearer token.
- [x] **MCP-CFG-004**: The system shall never write the MCP token to a log line, a debug rendering of the configuration (shown as `[REDACTED]`) or any HTTP response.
- [x] **MCP-CFG-005**: The system shall add the `/mcp` routes after the `/api` router's CORS layer, so the browser CORS policy does not apply to the server-to-server `/mcp` endpoint.
- [x] **MCP-CFG-006**: When `GET /api/v1/health` is called, the system shall report `mcp: "enabled"` when `/mcp` is mounted and `mcp: "disabled"` when it is not.

## Bearer guard

- [x] **MCP-AUTH-001**: The MCP bearer guard shall check every `/mcp` request before the MCP service sees it, and shall admit the request only when its `Authorization` header is valid ASCII, splits at the first space into a scheme equal to `bearer` ignoring case and a value that, trimmed, equals the configured token.
- [x] **MCP-AUTH-002**: The MCP bearer guard shall compare the presented token with the configured token in constant time.
- [x] **MCP-AUTH-003**: If a `/mcp` request is not admitted by the bearer guard (missing header, wrong token, a prefix of the token, another scheme, a non-ASCII header), then the system shall respond `401` with `WWW-Authenticate: Bearer realm="turfops-mcp"` and an empty body.

## Transport and server contract

- [x] **MCP-SRV-001**: The system shall serve `/mcp` over MCP streamable HTTP in stateless mode (no sessions), with plain JSON response bodies and no SSE keep-alive, so a server restart never strands a client.
- [x] **MCP-SRV-002**: The `/mcp` endpoint shall accept requests whatever their `Host` header (MCP's loopback-only host check disabled), because the gateway calls in by LAN hostname and the bearer token is the guard against DNS rebinding.
- [x] **MCP-SRV-003**: When an MCP client initializes, the server shall report the name `turfops`, the backend's package version, the `tools` and `resources` capabilities, and the doctrine (MCP-DOC) as its instructions, and shall offer no prompts.
- [x] **MCP-SRV-004**: Every MCP tool shall be annotated `read_only_hint = true`, and no MCP tool shall call a handler or query that creates, updates or deletes data.
- [x] **MCP-SRV-005**: Every MCP tool shall have a non-empty description that tells the model when to call it.
- [x] **MCP-SRV-006**: If a `/mcp` request body is larger than 1 MiB (the same limit as `/api/v1`), then the system shall reject it with `413 Payload Too Large` before any MCP tool runs.
- [x] **MCP-SRV-007**: When a `/mcp` request admitted by the bearer guard uses any HTTP method other than `POST` (the only method stateless streamable HTTP with JSON responses needs), the system shall respond `405 Method Not Allowed` with `Allow: POST`.

## Tool catalog

- [x] **MCP-TOOL-001**: The MCP server shall expose exactly these 18 tools: `lawn_profile`, `lawn_snapshot`, `current_conditions`, `recommendations`, `timing_windows`, `seasonal_plan`, `gdd`, `soil_temp_outlook`, `nitrogen_budget`, `weather_history`, `disease_risk`, `disease_detail`, `applications`, `soil_tests`, `soil_test_advice`, `products`, `shopping_list`, `plants`.
- [x] **MCP-TOOL-002**: Every MCP tool except `weather_history` (MCP-HIST) and date-bounded `applications` (MCP-TOOL-016, a read-only range query) shall answer by calling the REST handler behind the matching page as an in-process function, never an HTTP call to `/api`, and shall return that handler's body, shaped only as its spec states, as one JSON text content block with no output schema.
- [x] **MCP-TOOL-003**: Every MCP tool parameter shall be optional except `disease_detail`'s `slug`.
- [x] **MCP-TOOL-004**: When the MCP `lawn_profile` tool is called, it shall return the `GET /api/v1/profile` body as-is.
- [x] **MCP-TOOL-005**: When the MCP `current_conditions` tool is called, it shall return the `GET /api/v1/environmental` body with the 3-hourly `forecast.hourly` list removed and the daily forecast summaries kept.
- [x] **MCP-TOOL-006**: When the MCP `recommendations` tool is called, it shall return the `GET /api/v1/recommendations` body for its `include_inactive` parameter (default false), relaying the feed's recommendations unchanged and in the feed's order, without reordering, filtering, re-scoring or re-wording them (the feed's descending-severity order and its exclusion of soil-test advice are FEED-PIPE-009 and FEED-PIPE-018).
- [x] **MCP-TOOL-007**: When the MCP `timing_windows` tool is called, it shall return the `GET /api/v1/timing-windows` body with the daily soil chart `series` removed, unless `detail` is true, in which case it shall return the body whole.
- [x] **MCP-TOOL-008**: When the MCP `seasonal_plan` tool is called, it shall return the `GET /api/v1/seasonal-plan` body as-is for the given `year` (default: the handler's current year).
- [x] **MCP-TOOL-009**: When the MCP `gdd` tool is called, it shall return the `GET /api/v1/gdd` body for the given `year` with the `daily_history` list removed, and its description shall point to `weather_history` for daily values.
- [x] **MCP-TOOL-010**: When the MCP `soil_temp_outlook` tool is called, it shall return the `GET /api/v1/soil-temp-forecast` body as-is.
- [x] **MCP-TOOL-011**: When the MCP `nitrogen_budget` tool is called, it shall return the `GET /api/v1/nitrogen-budget` body as-is for the given `year`, including the per-application list.
- [x] **MCP-TOOL-012**: When the MCP `disease_risk` tool is called, it shall return `today`, `data_notes` and, per disease, its `slug`, `name`, `tier`, `tier_note`, `score_label`, `as_of`, `summary`, `factors`, management `action`, `headline`, `protection` and `notes`, and the one preventative FRAC option marked recommended, omitting the daily series, methodology and full programs.
- [x] **MCP-TOOL-013**: When the MCP `disease_detail` tool is called, it shall normalize `slug` (trimmed, lowercased, spaces and underscores turned into hyphens) and return `today`, `data_notes` and that disease's full `GET /api/v1/disease-risk` entry (daily series, factors, methodology, cultural practices, every preventative and curative option).
- [x] **MCP-TOOL-014**: If the MCP `disease_detail` tool's slug matches no disease, then it shall return the tool error "Unknown disease `<slug>`. Use one of: <slugs>", listing the slugs present in the disease-risk response.
- [x] **MCP-TOOL-015**: When the MCP `applications` tool is called without `since` or `until`, it shall return the `GET /api/v1/applications` list for its `type` filter, newest first, limited to `limit` entries (default 20, clamped to 1–100).
- [x] **MCP-TOOL-016**: When the MCP `applications` tool is called with `since` and/or `until` (YYYY-MM-DD, both inclusive, a missing side unbounded), it shall return the lawn profile's applications in that date range that match its `type` filter, newest first, limited to `limit` entries (default 20, clamped to 1–100).
- [x] **MCP-TOOL-017**: If the MCP `applications` tool's `since` or `until` is not a YYYY-MM-DD date, or `since` is after `until`, then it shall return a tool error saying "`<since|until>` must be a date as YYYY-MM-DD, got <value>" or "`since` must not be after `until`".
- [x] **MCP-TOOL-018**: If the MCP `applications` tool's `type` is not an application type, then it shall return the tool error "Unknown application type filter: <type>" (the `GET /api/v1/applications` wording), with the same text whether or not `since` or `until` is given.
- [x] **MCP-TOOL-019**: When the MCP `soil_tests` tool is called, it shall return the `GET /api/v1/soil-tests` list, newest first, limited to `limit` tests (default 5, clamped to 1–50).
- [x] **MCP-TOOL-020**: When the MCP `soil_test_advice` tool is called, it shall return the `GET /api/v1/soil-tests/recommendations` body as-is; it is the MCP server's only source of soil-test advice.
- [x] **MCP-TOOL-021**: When the MCP `products` tool is called, it shall return the `GET /api/v1/products` list for its `category` (matched to a product category ignoring case) and `include_archived` (default false) parameters, and its description shall call each product's `profile` AI-generated background, not fact.
- [x] **MCP-TOOL-022**: If the MCP `products` tool's `category` is not a product category, then it shall return the tool error "Unknown product category `<category>`. Use one of: <categories>", listing every product category by its enum name.
- [x] **MCP-TOOL-023**: The MCP `products` tool description shall say that a product's stored label rate may be reported only when the owner asks about that product, as the owner's recorded value with "verify against your label", and never inside advice.
- [x] **MCP-TOOL-024**: When the MCP `shopping_list` tool is called, it shall return the `GET /api/v1/inventory/shopping-list` body as-is.
- [x] **MCP-TOOL-025**: When the MCP `plants` tool is called, it shall return the `GET /api/v1/plants` list as-is, with each plant's full care plan.
- [x] **MCP-TOOL-026**: When the MCP `applications` tool is called with `since` and/or `until`, it shall apply its `type` filter and `limit` in the database query, so it never reads more than `limit` applications from the range.
- [x] **MCP-TOOL-027**: Every MCP trim of a handler body (`forecast.hourly`, timing `series`, gdd `daily_history`, the nitrogen budget's `applications`, and the fields the disease overview, detail and headline shapes pick) shall be tested against a body serialized from that handler's own response model, so that renaming or removing a field a trim names fails a test instead of silently leaving bulk in the reply or a picked field `null`.

## lawn_snapshot

- [x] **MCP-SNAP-001**: When the MCP `lawn_snapshot` tool is called, it shall fetch its parts concurrently from the profile, environmental, recommendations, applications, timing-windows, disease-risk and nitrogen-budget handlers, never from `GET /api/v1/dashboard` (which truncates the feed to three).
- [x] **MCP-SNAP-002**: The MCP `lawn_snapshot` result shall carry `profile` as-is, `conditions` with `forecast.hourly` removed, `recommendations` as the full active feed in the feed's order (the MCP `recommendations` tool's answer with `include_inactive` false), `recent_applications` as the 5 newest log entries, and `timing_windows` with `series` removed.
- [x] **MCP-SNAP-003**: The MCP `lawn_snapshot` result's `disease_risk` part shall carry `today`, `data_notes` and, per disease, only `slug`, `name`, `tier`, `tier_note`, `score_label`, `as_of`, management `action`, `headline` and `protected_through`.
- [x] **MCP-SNAP-004**: The MCP `lawn_snapshot` result's `nitrogen_budget` part shall carry the current year's budget totals without the per-application list.
- [x] **MCP-SNAP-005**: The MCP `lawn_snapshot` result shall carry `data_sources`, the data-source connection status under Data sync's key names (the weather lake under `soildata`).
- [x] **MCP-SNAP-006**: If one `lawn_snapshot` part's handler fails, then the tool shall set that part to `null`, add "<part> unavailable: <tool error text per MCP-ERR>" to `notes`, and return every other part.
- [x] **MCP-SNAP-007**: The MCP `lawn_snapshot` tool shall always return a successful result with a `notes` list, empty when no part failed.
- [x] **MCP-SNAP-008**: The MCP `lawn_snapshot` tool description shall tell the model to call it first for any question about what to do on the lawn now or this season.
- [x] **MCP-SNAP-009**: The MCP `lawn_snapshot` tool shall take `data_sources` from data sync's cached connection-probe results (data sync re-probes a source only when its cached result is older than 5 minutes), so a snapshot call does not make a live OpenWeatherMap request and Home Assistant ping every time.
- [x] **MCP-SNAP-010**: If the MCP `lawn_snapshot` tool cannot serialize the connection status, then it shall set `data_sources` to `null` and add "data_sources unavailable: <the internal-error text of MCP-ERR-005>" to `notes`, as for any failed part (MCP-SNAP-006).

## weather_history

- [x] **MCP-HIST-001**: The MCP `weather_history` tool shall cover the last `days` days (default 30, clamped to 1–120).
- [x] **MCP-HIST-002**: The MCP `weather_history` tool shall start its window at the server's local date (the app clock, governed by `TZ`) minus `days`, and shall report that start date as `start`.
- [x] **MCP-HIST-003**: The MCP `weather_history` tool shall build its rows from the lake's silver hourly data aggregated per local day and the memoized 15-year climate record, never from `GET /api/v1/historical` (whose chart downsampling makes daily rain sums wrong), with one row per date on or after `start` that either source has, in ascending date order.
- [x] **MCP-HIST-004**: Each MCP `weather_history` row shall carry `date`, `air_mean_f`, `air_min_f` and `air_max_f` (silver, °F, 0.1), `rh_mean_pct` (silver, 0.1), `rain_in` (silver mm ÷ 25.4, 0.01), `hours_observed` (silver hours covered), and `soil_5cm_f` and `gdd50` (climate record, 0.1), each `null` when its source has no value for that day.
- [x] **MCP-HIST-005**: When silver has no row for a day, the MCP `weather_history` tool shall fill that day's `air_mean_f` and `air_min_f` from the climate record's gold air values.
- [x] **MCP-HIST-006**: The MCP `weather_history` result shall carry `rain_total_in` (sum of the rows' `rain_in`, 0.01), `gdd50_total` (sum of the rows' `gdd50`, 0.1) and a fixed `note` explaining that a day with fewer than 24 hours observed is partial.
- [x] **MCP-HIST-007**: If no weather lake is configured, then the MCP `weather_history` tool shall return the weather lake's not-configured tool error ("Data source unavailable: " followed by the lake's not-configured sentence and the missing-data hint, per MCP-ERR-006); if either lake read fails, it shall return the tool error for that failure (MCP-ERR).

## Errors

- [x] **MCP-ERR-001**: When the handler or query behind an MCP tool fails, the tool shall return a tool error carrying the text below, never a JSON-RPC protocol error.
- [x] **MCP-ERR-002**: When an MCP tool fails with a not-found or invalid-data error, the tool error text shall be that error's message verbatim (messages written for the owner).
- [x] **MCP-ERR-003**: When an MCP tool fails because the weather lake could not be read, the tool error text shall say the NOAA weather data lake could not be read, so station soil readings, timing windows and disease risk are unavailable, followed by the missing-data hint, with no file path or SQL, and the system shall log the underlying error as a warning.
- [x] **MCP-ERR-004**: When an MCP tool fails because an outbound HTTP request failed, the tool error text shall say an external data source (forecast or Home Assistant) did not respond, followed by the missing-data hint, and the system shall log the underlying error as a warning.
- [x] **MCP-ERR-005**: When an MCP tool fails with any other error (database, configuration, I/O, JSON), the tool error text shall be "TurfOps hit an internal error answering this; details are in the server log." and the system shall log the underlying error as an error.
- [x] **MCP-ERR-006**: When an MCP tool fails because a data source is unavailable, the tool error text shall be "Data source unavailable: <plain-language name of the source and what is missing>." followed by the missing-data hint, and shall never include an upstream response body, a request URL or internal error text (such as a failed lake task's error), which the system shall log instead.
- [x] **MCP-ERR-007**: If an MCP tool's result body cannot be serialized to JSON, then the tool shall return a tool error with the internal-error text of MCP-ERR-005 and log the serialization error as an error, never a JSON-RPC protocol error.

## Doctrine

- [x] **MCP-DOC-001**: The MCP doctrine shall be one static text that describes how to use TurfOps and takes the lawn's particulars (grass, size, soil, irrigation) from the `lawn_profile` tool rather than stating them.
- [x] **MCP-DOC-002**: The MCP doctrine shall tell the model to call `lawn_snapshot` before answering what to do on the lawn now or this season, then use the narrower tools, never to guess a reading, date or product a tool can give, and to read the two resources (MCP-RES) when the owner asks why TurfOps says what it does.
- [x] **MCP-DOC-003**: The MCP doctrine shall describe the setting: a cool-season lawn in USDA zone 7a in southeast Pennsylvania; station data from a NOAA USCRN lake that lags about a day; an OpenWeatherMap forecast; observed and forecast values kept distinct; temperatures in °F; timing on 5 cm soil while the live reading and the grub window use 10 cm.
- [x] **MCP-DOC-004**: The MCP doctrine shall describe `today` in tool results as the lawn's local date (the server's `TZ`), and shall name no station-local or other second clock.
- [x] **MCP-DOC-005**: The MCP doctrine's rates rule shall tell the model: never state a product's label rate or any pesticide (fungicide, herbicide, insect control) rate, because the label governs; relay agronomic amounts (nutrient per 1,000 sq ft including the nitrogen budget, soil-test lime or sulfur, seed per 1,000 sq ft, mulch depth), optionally converted to an amount of a fertilizer or soil-amendment material but never of a material that is also a pesticide active (give the nutrient amount only); and report a shelf product's stored label rate only when the owner asks about that product, as the owner's recorded value with "verify against your label", never offered unprompted or used inside advice.
- [x] **MCP-DOC-006**: The MCP doctrine shall tell the model never to invent products, to prefer what is on the shelf, and when suggesting chemistry not on the shelf to name the active ingredient and, for a fungicide, the FRAC class, not a brand.
- [x] **MCP-DOC-007**: The MCP doctrine shall state that seeding and pre-emergent never share a season (a logged one makes the other `Blocked`), that fall pre-emergent is the alternative to fall seeding, and that core aeration belongs to the fall seeding window.
- [x] **MCP-DOC-008**: The MCP doctrine shall list the timing window states (NotYet, OpeningSoon, Open, Ideal, Closing, Closed, Done, Blocked) and boundary date sources (Observed, Tentative, Forecast, Typical) by their enum names, and shall say a boundary with `date: null` means "later than usual — not yet seen", never unknown, and must not be given an invented date.
- [x] **MCP-DOC-009**: The MCP doctrine shall state that only the disease tier is comparable across diseases, what `Protected` means, that red thread is treated with nitrogen rather than a spray, that the model must ask the owner what they see before recommending a curative (TurfOps has no symptom input), that FRAC classes rotate, and that the model calls `disease_detail` before naming a fungicide.
- [x] **MCP-DOC-010**: The MCP doctrine shall tell the model to cite an active recommendation (title and id) that already covers the question rather than re-derive it, give the meaning of each `inventory.state` (OnHand, Low, NotOnHand, Partial), and say that answered recommendations return when they escalate or their episode ends (`include_inactive` shows them).
- [x] **MCP-DOC-011**: The MCP doctrine shall tell the model to name the values that drove the advice and to say plainly when data is missing or stale: the environmental summary's `soil_stale` flag, `data_notes`, or a tool reporting a data source unavailable, without stating a staleness limit of its own.

## Resources

- [x] **MCP-RES-001**: The MCP server shall list exactly two `text/markdown` resources with their byte sizes: `turfops://docs/agronomy` (name `agronomy-methods`, from `docs/agronomy-methods.md`) and `turfops://docs/timing-windows` (name `timing-windows`, from `docs/timing-windows.md`).
- [x] **MCP-RES-002**: When an MCP client reads a listed resource URI, the server shall return that document's full markdown text as `text/markdown`.
- [x] **MCP-RES-003**: If an MCP client reads a resource URI that is not listed, then the server shall return a resource-not-found protocol error.
- [x] **MCP-RES-004**: The system shall embed both MCP resource documents in the binary at build time, so a missing document fails the build rather than the running server.
