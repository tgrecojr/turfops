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

Implementation: `backend/src/logic/data_sync.rs` (`DataSyncService`, `ConnectionStatus`: the I/O), `backend/src/logic/data_sync/sync_state.rs` (`SyncState`: staleness, assembly and the summary, with no I/O), `backend/src/logic/data_sync/probes.rs` (`ProbeCache`), `backend/src/api/timing/memo.rs` (the climate-record memo's age rules), `backend/src/state.rs` (`AppState`, `ClimateCache`), `backend/src/api/environmental.rs` (the two endpoints), `backend/src/api/health.rs` (connection status). The summary type lives in `backend/src/models/environmental.rs`.

## Boundaries

| Neighbor (HLD component) | Direction | Contract at the boundary |
|---|---|---|
| Weather data access | consumed | `WeatherLakeClient::fetch_summary()` → `EnvironmentalSummary` with `current` (newest value of each sensor within 24 h of the station's newest row, stamped with that row's time), 7-day averages, 7-day rain total and soil trend; `fetch_gdd_ytd(today)` → `Option<f64>`; `test_connection()` → readable gold parquet. `HomeAssistantClient::fetch_current()` → reading with `ambient_temp_f` / `humidity_percent` (per-entity failures already become `None`). `OpenWeatherMapClient::fetch_forecast()` → `WeatherForecast` stamped `fetched_at`. Query internals, unit conversion and local-date bucketing belong to Weather data access. |
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
| `climate_cache` | `ClimateCache` = `Arc<RwLock<Option<(DateTime<Utc>, Arc<ClimateRecord>)>>>` | 1 h memo of the multi-year climate record, with when its read completed |
| `mcp_enabled` | `bool` | Whether `/mcp` is mounted; reported by the health check |
| `clock` | `Clock` (`backend/src/clock.rs`) | The lawn's clock, resolved once from `TZ` (`Config::from_env`); every handler's "today" |

`DataSyncService` (`backend/src/logic/data_sync.rs`) holds the three source clients, the app's `Clock` (also handed to the OpenWeatherMap client for its day buckets), a `SyncState` and a `ProbeCache` (the last probe result per source with the time it was taken, see Connection Status). `SyncState` (`logic/data_sync/sync_state.rs`) holds the last good lake read, year-to-date GDD, the last good Home Assistant ambient temperature and humidity each with the time it was read, the sensor part as composed by the last sensor refresh, the last good forecast, and three wall-clock timestamps: the last sensor refresh, the last successful forecast fetch and the last forecast attempt. It does no I/O; every method takes `now`, so staleness, carry-forward, the 72 h gate and the forecast cap are unit-tested. It is built once at startup by `initialize()` (`backend/src/main.rs`), which only constructs clients. No connection is tested at startup, so a source that is down does not slow boot.

| Client | Configured when | Otherwise |
|---|---|---|
| Weather lake | Always (paths derive from `DATALAKE_ROOT`) | n/a: the field is an `Option` but is always `Some` |
| Home Assistant | `HA_TOKEN` non-empty | `None`; ambient fields stay empty |
| OpenWeatherMap | OWM config enabled and `OWM_API_KEY` non-empty | `None`; no forecast |

### Climate-record memo contract

The memo is a single slot shared by every caller of `api::timing::climate_record` (timing windows, the seasonal plan's timing activities via `plan_for`, and the MCP `weather_history` tool).

- **Key:** none. One record per process. The start date (`climate_start(today)`: Jan 1 of the lawn's local year, from `state.clock.today()`, minus `HISTORY_YEARS`) is not part of the key, so a record read just before the lawn's New Year is served for up to an hour into it with the old start.
- **TTL:** 1 h (`memo::FRESH_FOR`, `backend/src/api/timing/memo.rs`), measured from when the read completed.
- **Hit:** checked under the read lock; a fresh entry is returned as a shared `Arc`.
- **Miss:** the lake read runs with **no lock held**; the result is stored under a short write lock. Concurrent misses each read the lake and the last writer wins (no single-flight).
- **Failure:** a failed re-read is not memoized and leaves the slot unchanged. While the slot's record was read less than 24 h ago, the accessor serves it, marked as coming from the last good read, and Timing windows adds a data note saying so; the 1 h TTL still governs when the next re-read is attempted. Only when the slot is empty or its record is 24 h old or more (`memo::may_stand_in`) does the error reach the caller (SYNC-MEMO-002). `api::timing::climate_read` returns the record with `last_good_at` set in that case; `climate_record` (the plan's timing activities and `weather_history`) returns the record alone.
- **Lifetime:** in memory only, empty at startup. Nothing derived from it is written to Postgres, so the current season is recomputed from a record at most an hour old while the lake is readable, and at most a day old (and labelled as such) while it is not.

## Staleness and Refresh

| Part | Staleness limit | Timestamp | Set when |
|---|---|---|---|
| Sensors (lake soil + rain, Home Assistant ambient, GDD YTD) | 5 min (`SENSOR_STALE`) | the last sensor refresh | When a sensor refresh's summary has been stored, success or failure of any source |
| Forecast (OWM) | 30 min (`FORECAST_STALE`) | the last successful fetch | When a **successful** fetch's summary has been stored |

A part with no timestamp yet is stale. Each part is judged independently by `SyncState::due(now)`: `get_or_refresh()` refreshes only the parts that are due and returns `SyncState::summary(now)`; with nothing due it does no I/O. `force_refresh()` refreshes both.

**Fresh only once stored.** A refresh records the new freshness timestamps only after it has stored the summary it built, in the same step (SYNC-REFRESH-008). Every await (lake, Home Assistant, GDD, OWM) comes before that step, so a request dropped mid-refresh leaves both the old summary and the old timestamps in place, and the next request refreshes again. `DataSyncService::refresh` gathers every read first and then calls `store_sensors` / `store_forecast`, which compose the summary and set the timestamps together.

**Forecast retry interval.** A failed forecast fetch leaves the forecast part stale but records the attempt. The fetch is not tried again until 5 minutes after that attempt, so while OWM is down at most one request in five minutes pays its timeout under the lock; in between, the summary carries the last good forecast within its 6 h cap (SYNC-FAIL-008).

Who triggers a refresh: every handler that needs current conditions calls `get_or_refresh()` under the write lock: `/environmental`, `/dashboard`, `/recommendations`, `/inventory/shopping-list`, `/soil-temp-forecast`, `/disease-risk`, `/timing-windows`, `/seasonal-plan` (for the feed it assembles to judge the current season's rule states, so the plan and the Recommendations page see the same conditions), and through them the MCP tools. Handlers that only need the lake client (`/gdd`, `/historical`, the first half of `/soil-temp-forecast`) take the read lock and call `weather_client()`; they never refresh. `/seasonal-plan` also reads the lake under the read lock to fill its crossing cache.

### API endpoints owned

| Method | Path | Behavior | Response |
|---|---|---|---|
| GET | `/api/v1/environmental` | `get_or_refresh()` | `EnvironmentalSummary` |
| POST | `/api/v1/environmental/refresh` | `force_refresh()`: all sources, ignoring age | `EnvironmentalSummary` |

Neither returns a datasource error: the refresh absorbs every source failure and always returns `Ok`. The `Result` signature is kept, but the only way these endpoints fail is a panic or a dropped request.

## Assembling the Summary

A sensor refresh gathers the lake summary, the Home Assistant reading and year-to-date GDD, then `SyncState::store_sensors` builds the summary in this order:

1. **Lake.** `fetch_summary()` succeeds → its summary becomes the base (7-day averages, rain total, trend, `current`), and `soil_observed_at` = the lake reading's timestamp, which is the station's observation time.
   Fails → the base is the last good lake read, kept from an earlier refresh, so `soil_observed_at`, the soil values, aggregates and trend are carried forward.
2. **Station-age gate.** If `now − soil_observed_at > 72 h` (`SOIL_MAX_AGE`), the station's weather is withheld and a warning is logged: the soil values are not copied into the current reading, the 7-day averages (10 cm soil, station air, station humidity) and the 7-day precipitation total are set to `None`, and the soil trend to `Unknown` (SYNC-FAIL-004, SYNC-FAIL-009). The aggregates are computed over whatever lake rows fall in the last 7 days, so after 72 h without new rows they describe weather that is days old; withholding them keeps the rules that read them silent instead of acting on it. Otherwise the soil temperatures (5/10/20/50/100 cm), soil moistures and `precipitation_mm` are copied from the base's `current` into a fresh combined reading. Because this gate is applied after step 1, a carried-forward reading also ages out at 72 h. The same comparison sets the summary's `soil_stale` flag (true past 72 h; false within it or when `soil_observed_at` is absent), which is what every stale-soil banner reads (SYNC-MERGE-006).
3. **Home Assistant.** `fetch_current()` succeeds → `ambient_temp_f` and `humidity_percent` are taken from it where present, and each value read is kept as that field's last good value with its read time. When the read fails, or a field comes back empty, the field's last good value is used while it was read less than 1 h ago; after that, or when Home Assistant is unconfigured, the field is `None` (SYNC-FAIL-005). One dropout therefore does not lift the over-85°F fertilizer block, while an hour-old patio reading is no longer offered as current.
4. **Stamp.** The combined reading gets `timestamp = now` and `source = Cached`. It becomes `current`, and `last_updated = now`.
5. **GDD YTD.** `fetch_gdd_ytd(self.clock.today())` → `gdd_base50_ytd`: the sum of the gold layer's daily `gdd50` from Jan 1 of the lawn's local year through the lawn's local date. On failure the base's value stands: `None` after a successful lake read, the previous value after a failed one.

Without a sensor refresh, the sensor part is the one the last sensor refresh stored.

**Forecast.** When the forecast is stale, OWM is configured and no attempt was made in the last 5 minutes, `fetch_forecast()` runs; `store_forecast` keeps a success as the last good forecast and records every attempt's time. The forecast is attached when the summary is returned (`SyncState::summary(now)`), not when it is stored: the last good forecast is included **only if** `now − fetched_at < 6 h` (`FORECAST_MAX_AGE`). Older than that, the summary carries no forecast, whichever part was refreshed (SYNC-FAIL-007).

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
| `gdd_base50_ytd` | Lake gold `gdd50`, Jan 1 to today, the lawn's local year and date | |
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

`check_connections()` reports each source's status under the **read** lock, and reports `false` for unconfigured sources and failed probes. Each source's probe result, up or down, is kept with the time it was taken and reused for 5 minutes; only a configured source whose result is older or missing is probed again, sequentially (SYNC-CONN-006). The per-source results (`ProbeCache`, `logic/data_sync/probes.rs`) sit behind their own `std::sync::Mutex` inside the service, never held across a probe, so a status check never needs the service's write lock.

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
| Clock for the GDD year-to-date | The lawn's local year and date (the app's `Clock`, resolved from `TZ`) | The UTC year and date; a station-local date from the lake's UTC offset | HLD decision: every date gate and year boundary is a lawn-local calendar day on one configured clock. The UTC year rolls over hours before the lawn's New Year. |
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

1. **Write lock for fresh reads.** Every summary consumer takes the write lock even when nothing is stale (`get_or_refresh` takes `&mut self`), serializing all those handlers. Read-lock holders also do lake I/O while holding the guard (`/historical`, `/seasonal-plan`, `/gdd`) instead of cloning the client, as timing and disease do.
2. **Vestigial code.** `DataSyncService.pool` is unused (`#[allow(dead_code)]`); `weather_client` is always `Some`, so the lake-not-configured errors (`not_configured(Source::WeatherLake)`) in several handlers (e.g. `backend/src/api/historical.rs`, `backend/src/api/timing.rs`) are unreachable; `initialize` is `async` without awaiting; `get_or_refresh` and `force_refresh` return a `Result` that is never `Err`; the combined reading's `source` is `Cached` even when freshly read.
3. **No single-flight for the climate record.** Concurrent misses each run the 15-year lake read; the last writer wins.
4. **The I/O half is untested.** `DataSyncService` (which reads run, the probe calls) has no unit tests; the clients are concrete types. Everything it decides is in `SyncState` and `ProbeCache`, which are tested.

## References

- `backend/src/logic/data_sync.rs`: `DataSyncService` (the reads), `ConnectionStatus`
- `backend/src/logic/data_sync/sync_state.rs`: `SyncState`, staleness constants; `backend/src/logic/data_sync/probes.rs`: `ProbeCache`
- `backend/src/state.rs`: `AppState`, `ClimateCache`
- `backend/src/api/environmental.rs`: GET `/environmental`, POST `/environmental/refresh`
- `backend/src/api/health.rs`, `backend/src/api/dashboard.rs`: connection status consumers
- `backend/src/api/timing.rs`: `climate_read` / `climate_record` accessors (Timing windows); `backend/src/api/timing/memo.rs`: `FRESH_FOR`, `LAST_GOOD_FOR`, `last_good_note`
- `backend/src/datasources/weather.rs`, `backend/src/datasources/weather/latest.rs`, `backend/src/datasources/homeassistant.rs`, `backend/src/datasources/openweathermap.rs`: source contracts (Weather data access)
- `backend/src/models/environmental.rs`: `EnvironmentalSummary`, `EnvironmentalReading`
- `docs/high-level-design.md`: idle-cost and degradation goals
