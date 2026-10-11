---
parent: high-level-design
prefix: SYNC
---

# Data Sync

## Context and Design Philosophy

Data Sync is the demand-driven refresh layer between the external data sources and everything that reasons about current conditions. It owns one in-memory `EnvironmentalSummary` (current reading, 7-day aggregates, trend, GDD year-to-date, forecast) and decides when that summary is re-read from the weather lake, Home Assistant and OpenWeatherMap. It also owns the application's shared runtime state (`AppState`), including the slot that memoizes the 15-year climate record, and the datasource connection status shown on the dashboard and the health endpoint.

Three HLD commitments shape it:

- **Zero external calls when idle.** There is no background task, scheduler or poller. A refresh happens only inside a request that needs the summary, and only when the cached part it needs is older than its staleness limit.
- **Graceful degradation.** Losing one source removes only what depends on it. A refresh never fails as a whole: every source failure is logged and absorbed, and the summary is returned with that source's fields kept from the last good read (for a bounded time) or left empty. A failing source is not hammered: each retry waits for its own interval.
- **Withhold rather than invent.** Station weather that no longer describes the present (the feed has stopped) is not presented as current: neither the live soil reading nor the 7-day aggregates built from it. A forecast too old to describe the coming days is dropped instead of reused, and a carried-forward ambient reading expires after an hour.

Implementation: `backend/src/logic/data_sync.rs` (`DataSyncService`, `ConnectionStatus`), `backend/src/state.rs` (`AppState`, `ClimateCache`), `backend/src/api/environmental.rs` (the two endpoints), `backend/src/api/health.rs` (connection status). The summary type lives in `backend/src/models/environmental.rs`.

## Boundaries

| Neighbor (HLD component) | Direction | Contract at the boundary |
|---|---|---|
| Weather data access | consumed | `WeatherLakeClient::fetch_summary()` → `EnvironmentalSummary` with `current` (newest value of each sensor within 24 h of the station's newest row, stamped with that row's time), 7-day averages, 7-day rain total and soil trend; `fetch_gdd_ytd(year)` → `Option<f64>`; `test_connection()` → readable gold parquet. `HomeAssistantClient::fetch_current()` → reading with `ambient_temp_f` / `humidity_percent` (per-entity failures already become `None`). `OpenWeatherMapClient::fetch_forecast()` → `WeatherForecast` stamped `fetched_at`. Query internals, unit conversion and local-date bucketing belong to Weather data access. |
| Agronomic rules, Recommendation feed, Disease risk, Timing windows, MCP server | consumers | Receive a cloned `EnvironmentalSummary` from `get_or_refresh()`. Fields may be `None` at any time; consumers must degrade, not fail. |
| Timing windows | consumer of `AppState.climate_cache` | Owns what the climate record contains, the accessor (`api::timing::climate_record`) and the data note shown when the record comes from the last good read. Data Sync owns the slot and its memo contract (below), including the stale-record fallback. |
| Web UI | consumer | Renders `last_updated`, the stale-station banner (shown when the summary's `soil_stale` flag is true, dated from `soil_observed_at`; the UI applies no age limit of its own), the connection dots, and the Environmental page's refresh button. |

## Runtime State (`AppState`)

`AppState` (`backend/src/state.rs`) is the Axum state cloned into every handler and the MCP server. All members are cheap clones (`Arc` or pool handles).

| Field | Type | Purpose |
|---|---|---|
| `pool` | `sqlx::PgPool` | Application database |
| `rules_engine` | `Arc<RulesEngine>` | Stateless rules, built once |
| `sync_service` | `Arc<tokio::sync::RwLock<DataSyncService>>` | The one refresh service and its cached summary |
| `openrouter` | `Option<Arc<OpenRouterClient>>` | LLM client when configured |
| `climate_cache` | `ClimateCache` = `Arc<RwLock<Option<(Instant, Arc<ClimateRecord>)>>>` | 1 h memo of the multi-year climate record |
| `mcp_enabled` | `bool` | Whether `/mcp` is mounted; reported by the health check |

`DataSyncService` (`backend/src/logic/data_sync.rs`) holds the three source clients, the cached `current_summary`, the last good `current_forecast`, the last good Home Assistant ambient temperature and humidity each with the time it was read, two monotonic freshness timestamps (`last_sensor_refresh`, `last_forecast_refresh`), the time of the last failed forecast attempt, and the last probe result per source with the time it was taken (see Connection Status). It is built once at startup by `initialize()` (`backend/src/main.rs`), which only constructs clients. No connection is tested at startup, so a source that is down does not slow boot.

| Client | Configured when | Otherwise |
|---|---|---|
| Weather lake | Always (paths derive from `DATALAKE_ROOT`) | n/a: the field is an `Option` but is always `Some` |
| Home Assistant | `HA_TOKEN` non-empty | `None`; ambient fields stay empty |
| OpenWeatherMap | OWM config enabled and `OWM_API_KEY` non-empty | `None`; no forecast |

### Climate-record memo contract

The memo is a single slot shared by every caller of `api::timing::climate_record` (timing windows, the seasonal plan's timing activities via `plan_for`, and the MCP `weather_history` tool).

- **Key:** none. One record per process. The start date (Jan 1 of the current year minus `HISTORY_YEARS`) is not part of the key. The year is meant to be the server's local year (`TZ`); the accessor takes the UTC year (`backend/src/api/timing.rs:46`), a divergence (Deferred 14).
- **TTL:** 3600 s (`CLIMATE_CACHE_TTL`, `backend/src/api/timing.rs`), measured from when the read completed, on the monotonic clock.
- **Hit:** checked under the read lock; a fresh entry is returned as a shared `Arc`.
- **Miss:** the lake read runs with **no lock held**; the result is stored under a short write lock. Concurrent misses each read the lake and the last writer wins (no single-flight).
- **Failure:** a failed re-read is not memoized and leaves the slot unchanged. While the slot's record was read less than 24 h ago, the accessor serves it, marked as coming from the last good read, and Timing windows adds a data note saying so; the 1 h TTL still governs when the next re-read is attempted. Only when the slot is empty or its record is 24 h old or more does the error reach the caller (SYNC-MEMO-002). The accessor returns the error on every failed re-read today, a divergence (Deferred 13).
- **Lifetime:** in memory only, empty at startup. Nothing derived from it is written to Postgres, so the current season is recomputed from a record at most an hour old while the lake is readable, and at most a day old (and labelled as such) while it is not.

## Staleness and Refresh

| Part | Staleness limit | Timestamp | Set when |
|---|---|---|---|
| Sensors (lake soil + rain, Home Assistant ambient, GDD YTD) | 5 min (`SENSOR_STALENESS_SECS`) | `last_sensor_refresh` | When a sensor refresh's summary has been stored, success or failure of any source |
| Forecast (OWM) | 30 min (`FORECAST_STALENESS_SECS`) | `last_forecast_refresh` | When a **successful** fetch's summary has been stored |

A part with no timestamp yet is stale. Each part is judged independently: `get_or_refresh()` calls `refresh_internal(sensor_stale, forecast_stale)` when either is stale and otherwise returns a clone of the cached summary with no I/O. `force_refresh()` calls `refresh_internal(true, true)`.

**Fresh only once stored.** A refresh records the new freshness timestamps only after it has stored the summary it built, in the same step (SYNC-REFRESH-008). Every await (lake, Home Assistant, GDD, OWM) comes before that step, so a request dropped mid-refresh leaves both the old summary and the old timestamps in place, and the next request refreshes again. The code sets `last_sensor_refresh` before the GDD and OWM awaits, a divergence (Deferred 6).

**Forecast retry interval.** A failed forecast fetch leaves the forecast part stale but records the attempt. The fetch is not tried again until 5 minutes after that attempt, so while OWM is down at most one request in five minutes pays its timeout under the lock; in between, the summary carries the last good forecast within its 6 h cap (SYNC-FAIL-008). The code retries on every request while the part is stale, a divergence (Deferred 4).

Who triggers a refresh: every handler that needs current conditions calls `get_or_refresh()` under the write lock: `/environmental`, `/dashboard`, `/recommendations`, `/inventory/shopping-list`, `/soil-temp-forecast`, `/disease-risk`, `/timing-windows`, and through them the MCP tools. Handlers that only need the lake client (`/gdd`, `/historical`, `/seasonal-plan`, the first half of `/soil-temp-forecast`) take the read lock and call `weather_client()`; they never refresh.

### API endpoints owned

| Method | Path | Behavior | Response |
|---|---|---|---|
| GET | `/api/v1/environmental` | `get_or_refresh()` | `EnvironmentalSummary` |
| POST | `/api/v1/environmental/refresh` | `force_refresh()`: all sources, ignoring age | `EnvironmentalSummary` |

Neither returns a datasource error: `refresh_internal` absorbs every source failure and always returns `Ok`. The `Result` signature is kept, but the only way these endpoints fail is a panic or a dropped request.

## Assembling the Summary

A sensor refresh builds the summary in this order (`refresh_internal`, `backend/src/logic/data_sync.rs`):

1. **Lake.** `fetch_summary()` succeeds → its summary becomes the base (7-day averages, rain total, trend, `current`), and `soil_observed_at` = the lake reading's timestamp, which is the station's observation time.
   Fails → the base is the previous cached summary with `forecast` cleared, and `soil_observed_at` keeps its previous value. The last good soil values, aggregates and GDD are carried forward.
2. **Station-age gate.** If `now − soil_observed_at > 72 h` (`SOIL_MAX_AGE_HOURS`), the station's weather is withheld and a warning is logged: the soil values are not copied into the current reading, the 7-day averages (10 cm soil, station air, station humidity) and the 7-day precipitation total are set to `None`, and the soil trend to `Unknown` (SYNC-FAIL-004, SYNC-FAIL-009). The aggregates are computed over whatever lake rows fall in the last 7 days, so after 72 h without new rows they describe weather that is days old; withholding them keeps the rules that read them silent instead of acting on it. Otherwise the soil temperatures (5/10/20/50/100 cm), soil moistures and `precipitation_mm` are copied from the base's `current` into a fresh combined reading. Because this gate is applied after step 1, a carried-forward reading also ages out at 72 h. The same comparison sets the summary's `soil_stale` flag (true past 72 h; false within it or when `soil_observed_at` is absent), which is what every stale-soil banner reads (SYNC-MERGE-006). The code withholds only the current reading, a divergence (Deferred 2), and sends no `soil_stale` flag (Deferred 11).
3. **Home Assistant.** `fetch_current()` succeeds → `ambient_temp_f` and `humidity_percent` are taken from it where present, and each value read is kept as that field's last good value with its read time. When the read fails, or a field comes back empty, the field's last good value is used while it was read less than 1 h ago; after that, or when Home Assistant is unconfigured, the field is `None` (SYNC-FAIL-005). One dropout therefore does not lift the over-85°F fertilizer block, while an hour-old patio reading is no longer offered as current. The code carries nothing forward, a divergence (Deferred 3).
4. **Stamp.** The combined reading gets `timestamp = now` and `source = Cached`. It replaces `current` and `last_updated = now`.
5. **GDD YTD.** `fetch_gdd_ytd(year)` → `gdd_base50_ytd`: the sum of the gold layer's daily `gdd50` from Jan 1 of the server's local year (`TZ`) through the server's local date. On failure the base's value stands: `None` after a successful lake read, the previous value after a failed one. The code takes the **UTC** year and date instead (`backend/src/logic/data_sync.rs:216`, `backend/src/datasources/weather.rs:241`), a bug (Deferred 7).

Without a sensor refresh, the base is the cached summary unchanged.

**Forecast.** When the forecast is stale, OWM is configured and no failed attempt was made in the last 5 minutes, a successful `fetch_forecast()` sets `summary.forecast` and `current_forecast`; a failed one records the attempt time. If `summary.forecast` is still empty afterwards (fetch failed, OWM not configured, or the sensor branch cleared it), the last good `current_forecast` is reused **only if** `now − fetched_at < 6 h` (`FORECAST_MAX_AGE_HOURS`). Older than that, the summary carries no forecast. Finally, in one step with no await, the result is stored as `current_summary`, `last_sensor_refresh` (after a sensor refresh) and `last_forecast_refresh` (after a successful fetch) are set to now, and the summary is returned.

### Field provenance

| Summary field | Source | Notes |
|---|---|---|
| `current.soil_temp_*_f`, `current.soil_moisture_*`, `current.precipitation_mm` | Lake silver (per-sensor newest within 24 h) | Empty when the station is > 72 h old |
| `current.ambient_temp_f`, `current.humidity_percent` | Home Assistant | After a failed or empty read, the field's last good value while under 1 h old; empty after that or when HA is unconfigured. The lake's air temperature and humidity are not used here |
| `current.timestamp` | Time of the refresh | Not the observation time |
| `soil_observed_at` | Lake observation time | Serialized only when present; the banners show it as the date the feed stopped |
| `soil_stale` | `soil_observed_at` more than 72 h old at the sensor refresh | The one place the 72 h limit is applied for display; every stale-soil banner reads it |
| `soil_temp_7day_avg_f` (10 cm), `ambient_temp_7day_avg_f`, `humidity_7day_avg`, `precipitation_7day_total_mm`, `soil_temp_trend` | Lake silver, last 7 days | Station air and humidity, not the patio sensor. Empty (trend `Unknown`) when the station is > 72 h old |
| `last_updated` | Time of the last sensor refresh | |
| `gdd_base50_ytd` | Lake gold `gdd50`, Jan 1 to today, local year and date | Code uses the UTC year and date (Deferred 7) |
| `forecast` | OWM | ≤ 30 min since fetch when healthy; ≤ 6 h after failures |

## Failure and Degradation

| Failure | Effect on the summary | Recovery |
|---|---|---|
| Lake read fails | Last good soil, aggregates, trend and GDD kept; `soil_observed_at` unchanged | Retried at the next sensor refresh (5 min) |
| Lake current but station feed stopped (> 72 h) | Current soil and rain, the 7-day aggregates and the trend withheld; `soil_stale` true; `soil_observed_at` still reported so the UI can say when it stopped | Automatic when the feed resumes |
| Lake never read successfully | Empty soil, aggregates and GDD | Next sensor refresh |
| Home Assistant fails (or a field reads empty) | Each field's last good value kept while under 1 h old, then `None` | Next sensor refresh |
| Home Assistant unconfigured | Ambient temperature and humidity `None` | n/a |
| GDD read fails | `gdd_base50_ytd` `None` (or the previous value if the lake read also failed); GDD-enhanced rules fall back | Next sensor refresh |
| OWM fails | Last forecast kept while `fetched_at` < 6 h; forecast stays stale, but the fetch is not retried for 5 min after the failed attempt | First successful fetch |
| OWM unconfigured | No forecast; forecast-dependent features degrade | n/a |

Errors are logged at `warn` with the source's error text and never returned to the client.

## Concurrency

`DataSyncService` sits behind one `tokio::sync::RwLock`. `get_or_refresh()` and `force_refresh()` take `&mut self`, so every caller that may refresh takes the **write** lock, even when the summary is fresh. The lock is held across all external awaits of a refresh: lake summary, two Home Assistant entity reads, GDD, and the OWM fetch. The HTTP clients' timeouts (connect 5 s; request 10 s for HA, 15 s for OWM) bound a refresh at roughly 35 s of network waiting plus the lake reads.

Consequences:

- **Two simultaneous refresh triggers.** The first caller refreshes. The second waits on the lock, then finds both parts fresh and returns the new summary with no I/O. Refreshes are never duplicated and never interleave. The MCP `lawn_snapshot` tool, which runs several handlers concurrently with `tokio::join!`, gets one refresh this way.
- **Readers wait behind a refresh.** `tokio`'s RwLock is fair, so read-lock users (`/gdd`, `/historical`, `/seasonal-plan`, `/health`, connection checks) queue behind a refresh in progress. Read-lock holders that do lake I/O while holding the lock (`/historical`, `/seasonal-plan`, `/gdd`, and the start of `/soil-temp-forecast`, which drops its read guard before taking the write lock) also delay a pending refresh.
- **Timing windows and disease risk** take the write lock only for `get_or_refresh()`, clone the lake client and forecast, and release it before their own lake reads.

## Connection Status

`check_connections()` reports each source's status under the **read** lock, and reports `false` for unconfigured sources and failed probes. Each source's probe result, up or down, is kept with the time it was taken and reused for 5 minutes; only a configured source whose result is older or missing is probed again, sequentially (SYNC-CONN-006). The per-source results sit behind their own small lock inside the service, so a status check never needs the service's write lock. The code probes every configured source on every call, a divergence (Deferred 9).

| JSON key | Probe | Notes |
|---|---|---|
| `soildata` | DuckDB `SELECT 1` on the gold parquet | Key name kept for frontend compatibility; it reports the weather lake. Silver readability is not probed. |
| `homeassistant` | `GET {HA_URL}/api/` with the bearer token | 2xx = up |
| `openweathermap` | `GET /weather` (current conditions) with the API key | 2xx = up; a real OWM API call |

Callers: `GET /api/v1/health` (with a 2 s database ping; `status` is `ok`/`degraded` on the database alone), `GET /api/v1/dashboard` (after its `get_or_refresh()` and after releasing the write lock), and the MCP `lawn_snapshot` tool. With the 5-minute reuse, dashboard loads, health checks and `lawn_snapshot` calls together cost at most one probe per source per 5 minutes, which also bounds the OWM quota the probe spends. Idle cost is zero, because nothing calls them unless a page or client does. A status can therefore lag a source's recovery or outage by up to 5 minutes; the dots are informational and drive no behavior. The compose file defines no health check for the app container.

## Decisions & Alternatives

| Decision | Chosen | Alternatives Considered | Rationale |
|---|---|---|---|
| When to refresh | On demand inside the request, per-part staleness (sensors 5 min, forecast 30 min) | Background polling task; refresh on every request | HLD goal: zero external calls when idle; the forecast changes far more slowly than the station. |
| Raw weather caching | Keep only the in-memory summary; re-read the lake on each sensor refresh | Postgres cache of observations or of the summary | Owner decision (lake migration): lake parquet reads are local and fast, and a copy goes stale. |
| Startup | Construct clients without testing them | Test connections at boot | A source that is down must not slow or block startup. [inferred from the `initialize` doc comment] |
| Failed lake read | Keep the last good soil summary | Clear soil until the next success | Owner decision (functional review): a failed read must not silently silence every soil rule for 5 minutes. |
| Stopped station feed | Withhold current soil past 72 h, keep reporting `soil_observed_at` | Show the old reading stamped "now"; hide `soil_observed_at` too | Owner decision (functional review, lake staleness): a weeks-old reading would drive today's advice. The lake normally lags about a day, so 72 h allows for that lag. The tenet "withhold rather than invent" applies. |
| 7-day aggregates when the feed has stopped | Withhold the 7-day averages, rain total and trend past 72 h too | Keep computing them from whatever rows fall in the last 7 days; withhold only past 7 days | Rules read the averages and rain total as this week's weather. Between 72 h and 7 days they describe days-old weather, and a rule silent for lack of data is safer than one acting on it. |
| Failed forecast fetch | Reuse the last forecast while under 6 h old | Drop immediately; reuse indefinitely | One OWM failure should not silence every forecast rule, but a forecast older than a few hours no longer describes the coming days. [inferred from code comments] |
| Home Assistant failure | Keep each field's last good value for up to 1 h, then `None` | `None` at once; keep the last value indefinitely; fall back to station air | The over-85°F fertilizer block reads the patio temperature, so dropping it on one failed read would lift the block mid-afternoon. An hour bridges a dropout; beyond that a patio reading no longer describes the yard. |
| Current ambient source | Home Assistant only; the lake's station air/humidity are not used for `current` | Fall back to station air when HA is missing | [inferred] The patio sensor is the yard's own reading; the 7-day aggregates already use the station. |
| Refresh concurrency | One `RwLock`, write lock held across the whole refresh | Single-flight future; lock-free snapshot swap | Simplest way to guarantee one refresh at a time and no duplicate external calls; a single-user app tolerates the serialization. [inferred] |
| Forecast retry after a failure | Wait 5 min after a failed attempt before fetching again | Retry on every request while stale; exponential backoff | Each retry can hold the write lock for the 15 s OWM timeout, so retrying on every request stalls every page during an outage. The last good forecast covers the gap for up to 6 h, and 5 min matches the sensor interval. |
| When a part counts as fresh | Set the freshness timestamps in the same step that stores the new summary, after every await | Set them as each source finishes | A request dropped mid-refresh must not leave the old summary marked fresh for 5 minutes; with the timestamps set last, an abandoned refresh simply runs again on the next request. |
| Errors from a refresh | Absorb every source error; always return a summary | Fail the request when a source fails | HLD goal: losing one source degrades only dependent features. [inferred] |
| Clock for the GDD year-to-date | The server's local year and date (`chrono::Local`, set by `TZ` to the lawn's timezone) | The UTC year and date; a station-local date from the lake's UTC offset | HLD decision: every date gate and year boundary is a lawn-local calendar day on one configured clock. The UTC year rolls over hours before the lawn's New Year. |
| Climate-record memo | One in-memory slot, 1 h TTL, not in Postgres | Persist crossings for the current season; no memo | HLD decision: the 15-year read is the expensive one, and the current season must stay live. |
| Connection probes | Reuse each source's probe result, up or down, for 5 min | Probe on every call; derive status from the last refresh outcome | Dashboard loads, `/health` and `lawn_snapshot` would otherwise call Home Assistant and OWM (spending quota) every time. A probe still tests the connection directly, and a status at most 5 min old is enough for indicator dots. |
| Stale-soil threshold for display | Backend sends `soil_stale`; banners show it without an age limit of their own | Each banner compares `soil_observed_at` with its own 72 h | One limit in one place: the banner and the withholding of soil data cannot drift apart. |
| Climate record when a re-read fails | Serve the last memoized record for up to 24 h, with a data note | Fail the caller once the 1 h memo expires; serve the old record indefinitely | The climate record is 15 years of history, so a day-old copy changes little, while failing would blank the Timing page during a brief lake outage. Past a day the current season's observed days are too far behind to present. |
| Connection-status key | Keep `soildata` for the lake | Rename to `weatherlake` | Owner decision (lake migration): frontend compatibility. |

## Open Questions & Future Decisions

### Resolved

1. ✅ Raw weather is not cached in Postgres; the summary is rebuilt from the lake on each sensor refresh (owner decision, lake migration).
2. ✅ A failed lake read keeps the last good soil summary; a station feed older than 72 h is not used as current, and its observation time is surfaced (owner decision, functional review).

### Deferred

1. **Security: the OpenWeatherMap API key reaches the logs.** A transport-level `fetch_forecast` failure (DNS, connect, timeout) becomes `TurfOpsError::DataSourceUnavailable(format!("OpenWeatherMap: {}", e))` from a `reqwest::Error` (`backend/src/datasources/openweathermap.rs:108-110`). reqwest's `Display` appends ` for url (…)`, and that URL carries `appid=<key>`. Data Sync logs the error text at `warn` (`backend/src/logic/data_sync.rs:242`). This breaks the project rule against secrets in logs. The fix belongs in the OpenWeatherMap client (strip the URL from the error, e.g. `without_url()`), owned by Weather data access (`docs/intent/weather-data/weather-sources/`); Data Sync needs no change once the client's errors are clean. The connection probe builds the same kind of error (`openweathermap.rs:141-143`) but `check_connections()` discards it (`data_sync.rs:115`), so only the forecast path leaks.
2. **Divergence: the 7-day aggregates outlive the 72 h gate** (SYNC-FAIL-009). Intent: past 72 h without a new observation, the 7-day averages and rain total are empty and the trend `Unknown`, alongside the withheld current reading. The gate (`data_sync.rs:170-174`) withholds only `current`; the aggregates are still computed from whatever lake rows fall in the last 7 days (`backend/src/datasources/weather.rs:113-139`), so between 72 h and 7 days of feed outage they describe old weather.
3. **Divergence: Home Assistant ambient is not carried forward** (SYNC-FAIL-005). Intent: after a failed or empty read, each ambient field keeps its last good value while under 1 h old, then goes empty. The code leaves both fields `None` on any failure (`data_sync.rs:204-206`) and keeps no last good ambient value, so one dropout lifts the over-85°F fertilizer block.
4. **Divergence: OWM is retried on every request while failing** (SYNC-FAIL-008). Intent: after a failed forecast fetch, no new attempt for 5 minutes. `last_forecast_refresh` is set only on success (`data_sync.rs:238`) and no attempt time is kept, so while OWM is failing every request that takes the write lock retries it, each costing up to the 15 s timeout under the lock.
5. **Forecast age cap bypass.** When sensors are fresh and only the forecast refresh runs and fails, the base summary is the cached one with its old forecast already in it (`data_sync.rs:229`), so the 6 h filter at `data_sync.rs:250-255` never runs. This lasts until the next sensor refresh (≤ 5 min), so the overshoot is small.
6. **Divergence: freshness recorded before the summary is stored** (SYNC-REFRESH-008). Intent: the freshness timestamps are set in the same step that stores the new summary, after every await. `last_sensor_refresh` is set at `data_sync.rs:213`, before the GDD (`:218`) and OWM (`:234`) awaits, and `current_summary` is assigned only at `:258`. If the request future is dropped during those awaits, the timestamps say fresh while the cached summary is the old one, for up to 5 minutes.
7. **Bug: GDD year-to-date uses the UTC year and date.** The sensor refresh passes `Utc::now().year()` (`data_sync.rs:216`) and `fetch_gdd_ytd` sums to `Utc::now().date_naive()` (`weather.rs:241`). Intent is the server's local year and date. On the evening of Dec 31 in the lawn's timezone, UTC is already in the new year, so the range is Jan 1 to Jan 1 of a year the lake has no rows for yet and GDD YTD reads as `None` (`weather.rs:245-247`) instead of the full-season total; the GDD-enhanced rules lose their GDD input for those hours. Every other evening the range end is one day past the local date, which is harmless only because the lake lags about a day. The intended value is specified by Agronomic rules (RULE-GDD-002); the fix lands in this component's refresh and `fetch_gdd_ytd`.
8. **Write lock for fresh reads.** Every summary consumer takes the write lock even when nothing is stale (`get_or_refresh` takes `&mut self`, `data_sync.rs:86`), serializing all those handlers. Read-lock holders also do lake I/O while holding the guard (`/historical`, `/seasonal-plan`, `/gdd`) instead of cloning the client, as timing and disease do.
9. **Divergence: connection probes run on every call** (SYNC-CONN-006). Intent: each source's probe result is reused for 5 minutes. Each dashboard view (`backend/src/api/dashboard.rs:43`), health call (`backend/src/api/health.rs:29`) and `lawn_snapshot` (`backend/src/mcp/tools/lawn.rs:81`) runs `check_connections()` (`data_sync.rs:103-119`), which probes every configured source, making HA and OWM requests; the OWM probe spends API quota.
10. **Vestigial code.** `DataSyncService.pool` is unused (`#[allow(dead_code)]`, `data_sync.rs:22-23`); `weather_client` is always `Some` (`data_sync.rs:66`), so the "weather data lake not configured" errors in several handlers (e.g. `backend/src/api/historical.rs:39-40`, `backend/src/api/timing.rs:61`) are unreachable; `initialize` is `async` without awaiting (`data_sync.rs:37`); `refresh_internal` returns a `Result` that is never `Err`; the combined reading's `source` is `Cached` even when freshly read (`data_sync.rs:146`).
11. **Divergence: the stale-soil threshold is duplicated in the UI** (SYNC-MERGE-006). Intent: the summary carries a `soil_stale` flag computed with data sync's 72 h limit, and every stale-soil banner reads that flag. The summary has no such field (`backend/src/models/environmental.rs`); the dashboard banner hard-codes its own 72 h (`frontend/src/pages/Dashboard.tsx:39` `SOIL_STALE_MS`), independent of `SOIL_MAX_AGE_HOURS` (`data_sync.rs:13`), so the two can drift. The banners themselves are specified by FEED-UI-009 (dashboard) and WX-VIEW-PAGE-008 (Environmental page), which must read the flag.
12. **No tests.** `DataSyncService` has no unit tests for staleness, carry-forward, the 72 h gate or the forecast cap. The clients are concrete types (`data_sync.rs:24-26`), so the module cannot be tested without a seam (trait or injected fetchers).
13. **Divergence: no stale-record fallback for the climate record** (SYNC-MEMO-002). Intent: when a re-read fails, the last memoized record is served while under 24 h old, marked as from the last good read so Timing windows adds a data note. `climate_record` returns the lake error on any failed re-read once the 1 h memo has expired (`backend/src/api/timing.rs:39-51`), and the Timing page fails. Separately, and still open: concurrent misses each run the 15-year lake read (no single-flight).
14. **Divergence: climate-record start year from UTC.** `climate_record` dates the history start from `Utc::now().year()` (`backend/src/api/timing.rs:46`) instead of the server's local year. For the hours between UTC and local New Year the record starts a year later than intended, and because the memo has no key, a record read just before local New Year is served for up to an hour into it with the old start. The accessor is owned by Timing windows; the slot's contract is owned here.

## References

- `backend/src/logic/data_sync.rs`: `DataSyncService`, staleness constants, `refresh_internal`, `ConnectionStatus`
- `backend/src/state.rs`: `AppState`, `ClimateCache`
- `backend/src/api/environmental.rs`: GET `/environmental`, POST `/environmental/refresh`
- `backend/src/api/health.rs`, `backend/src/api/dashboard.rs`: connection status consumers
- `backend/src/api/timing.rs`: `climate_record` accessor and `CLIMATE_CACHE_TTL` (Timing windows)
- `backend/src/datasources/weather.rs`, `backend/src/datasources/weather/latest.rs`, `backend/src/datasources/homeassistant.rs`, `backend/src/datasources/openweathermap.rs`: source contracts (Weather data access)
- `backend/src/models/environmental.rs`: `EnvironmentalSummary`, `EnvironmentalReading`
- `docs/high-level-design.md`: idle-cost and degradation goals
