# Data Sync — EARS Specs

Design: [data-sync-design.md](data-sync-design.md). Prefix `SYNC`. Facets: `REFRESH` (staleness limits and demand-driven refresh), `MERGE` (assembling the cached environmental summary), `FAIL` (failures, last-good values, the 72 h soil cut-off, forecast reuse, retry), `CONN` (connection checks and `/health`), `MEMO` (the shared climate-record slot in runtime state).

"The summary" is data sync's one in-memory `EnvironmentalSummary`. Its "sensor part" is the lake's live soil reading, 7-day aggregates and soil trend, the Home Assistant ambient reading and year-to-date GDD; its "forecast part" is the OpenWeatherMap forecast. A "sensor refresh" re-reads the sensor part; a "forecast refresh" re-fetches the forecast. The live reading per sensor and each source's query contract are owned by Weather sources (`WX-SRC-*`); the `/environmental` endpoints' contract is WX-VIEW-API-001.

## Demand-driven refresh

- [x] **SYNC-REFRESH-001**: The system shall run no background task, scheduler or poller for the weather lake, Home Assistant or OpenWeatherMap, and shall contact them only while serving a request that needs current conditions or connection status.
- [x] **SYNC-REFRESH-002**: Data sync shall treat the summary's sensor part as stale 5 minutes after its last sensor refresh and its forecast part as stale 30 minutes after its last successful forecast fetch, and shall treat a part that has never been refreshed as stale.
- [x] **SYNC-REFRESH-003**: When a request needs current conditions and neither part of the summary is stale, data sync shall return the cached summary without contacting any source.
- [x] **SYNC-REFRESH-004**: When a request needs current conditions and one part of the summary is stale, data sync shall refresh only that part's sources and keep the other part as cached.
- [x] **SYNC-REFRESH-005**: The handlers for `/environmental`, `/dashboard`, `/recommendations`, `/inventory/shopping-list`, `/soil-temp-forecast`, `/disease-risk`, `/timing-windows` and `/seasonal-plan` (the last for the feed it assembles to judge the current season's rule states, PLAN-ACT-021), and the MCP tools that call them, shall obtain current conditions only through data sync's refresh-on-demand; no other handler shall trigger a refresh.
- [x] **SYNC-REFRESH-006**: While a refresh is in progress, data sync shall make every other request that may refresh wait for it and then reuse its result, so simultaneous requests (including the concurrent parts of the MCP `lawn_snapshot` tool) cause at most one read of each source.
- [x] **SYNC-REFRESH-007**: When the server starts, data sync shall set up its source clients without contacting any source, so a source that is down does not delay startup.
- [x] **SYNC-REFRESH-008**: Data sync shall mark the summary's sensor or forecast part as refreshed only once the summary built by that refresh has been stored, so a refresh abandoned partway (for example, the request is dropped while a source read is pending) leaves the part stale rather than leaving the previous summary counted as fresh.
- [x] **SYNC-REFRESH-009**: Data sync shall take "now" from the app clock for every judgment it makes (which part is stale, when a failed fetch may be retried, how old a probe or the station's observation is, and where the lake's 7-day window ends), never from the system time.

## Assembling the summary

- [x] **SYNC-MERGE-001**: When a sensor refresh reads the lake successfully, data sync shall take the 7-day means, the 7-day precipitation total, the soil trend and the live soil reading from that read, and set `soil_observed_at` to the lake reading's observation time.
- [x] **SYNC-MERGE-002**: When a sensor refresh builds the summary's current reading, data sync shall take soil temperatures (5–100 cm), soil moistures and precipitation from the lake's live reading, and ambient temperature and humidity from Home Assistant only, never from the station's air temperature or humidity.
- [x] **SYNC-MERGE-003**: When a sensor refresh completes, data sync shall stamp the current reading's `timestamp` and the summary's `last_updated` with the time of the refresh, leaving the station's observation time only in `soil_observed_at`.
- [x] **SYNC-MERGE-004**: When a sensor refresh runs, data sync shall re-read year-to-date GDD into the summary along with the sensor part (the value itself is specified by RULE-GDD-002).
- [x] **SYNC-MERGE-005**: When a forecast fetch succeeds, data sync shall put that forecast in the summary and keep it as the last good forecast.
- [x] **SYNC-MERGE-006**: When a sensor refresh completes, data sync shall set the summary's `soil_stale` flag to true when `soil_observed_at` is more than 72 hours old (the SYNC-FAIL-004 limit) and to false when it is within 72 hours or absent, so every web UI stale-soil banner reads this flag and the 72-hour limit is defined only in data sync.

## Failures and degradation

- [x] **SYNC-FAIL-001**: If any source fails during a refresh, then data sync shall log a warning and still return a summary, so no caller of a refresh (including `GET /api/v1/environmental` and `POST /api/v1/environmental/refresh`) receives a data-source error from it.
- [x] **SYNC-FAIL-002**: If the lake read fails during a sensor refresh, then data sync shall keep the previous summary's soil reading, 7-day aggregates, soil trend and `soil_observed_at` rather than clear them.
- [x] **SYNC-FAIL-003**: If the year-to-date GDD read fails during a sensor refresh, then data sync shall leave GDD empty when the lake read succeeded, and keep the previous GDD when the lake read also failed.
- [x] **SYNC-FAIL-004**: While the lake's newest observation (`soil_observed_at`) is more than 72 hours old, data sync shall leave the current reading's soil temperatures, soil moistures and precipitation empty, keep reporting `soil_observed_at`, and log a warning, whether that observation came from this refresh's read or was carried forward after failed reads.
- [x] **SYNC-FAIL-005**: If a Home Assistant read of ambient temperature or humidity fails (or returns no value) during a sensor refresh, then data sync shall keep that field's last good Home Assistant value in the current reading while that value was read less than 1 hour ago, and leave the field empty once it is 1 hour old or when Home Assistant is not configured.
- [x] **SYNC-FAIL-006**: If a forecast fetch fails or OpenWeatherMap is not configured, then data sync shall serve the last good forecast in the summary only while it was fetched less than 6 hours ago, and no forecast otherwise.
- [x] **SYNC-FAIL-007**: The summary data sync returns shall never carry a forecast fetched 6 or more hours ago, including when only the forecast part was refreshed and its fetch failed.
- [x] **SYNC-FAIL-008**: If a source fails during a refresh, then data sync shall retry it on a later request: a failed lake, GDD or Home Assistant read at the next sensor refresh (5 minutes), and a failed OpenWeatherMap forecast fetch at the first request that needs current conditions 5 minutes or more after the failed attempt, never on every request while the forecast part stays stale.
- [x] **SYNC-FAIL-009**: While the lake's newest observation (`soil_observed_at`) is more than 72 hours old, data sync shall also leave the summary's 7-day averages (10 cm soil, station air temperature, station humidity) and 7-day precipitation total empty and report the soil trend as `Unknown`, so rules that use them stay silent rather than act on stale weather.

## Connection status and health

- [x] **SYNC-CONN-001**: When data sync reports connection status, it shall report `soildata` (the weather lake, probed per WX-VIEW-PROBE-001), `homeassistant` (`GET {HA_URL}/api/` with the bearer token answers 2xx) and `openweathermap` (an OpenWeatherMap current-weather request answers 2xx), reporting `false` for an unconfigured source or a failed probe and never failing itself.
- [x] **SYNC-CONN-002**: When connection status is requested, data sync shall leave the summary and its staleness timers unchanged.
- [x] **SYNC-CONN-003**: When `GET /api/v1/health` is called, the system shall respond with `status`, `version`, `database`, the connection status under `datasources`, and `mcp` (`enabled` when `/mcp` is mounted, else `disabled`).
- [x] **SYNC-CONN-004**: `GET /api/v1/health` shall report `status` `ok` and `database` true when the database answers `SELECT 1` within 2 seconds, and `degraded` and false otherwise, and shall not let an external source's outage change `status`.
- [x] **SYNC-CONN-005**: `GET /api/v1/dashboard` (as `connections`) and the MCP `lawn_snapshot` tool (as `data_sources`) shall include the connection status, checked after their summary is refreshed.
- [x] **SYNC-CONN-006**: When connection status is requested, data sync shall reuse each source's (weather lake, Home Assistant, OpenWeatherMap) last probe result, up or down, while it is less than 5 minutes old and probe only the configured sources whose result is older or missing, so `/health`, dashboard loads and the MCP `lawn_snapshot` tool do not call the upstreams on every request.

## Climate-record slot

- [x] **SYNC-MEMO-001**: The system shall keep one in-process climate-record slot, empty at startup, shared by every caller of the climate record (timing windows, the seasonal plan's timing activities, the MCP `weather_history` tool), measuring a record's age from when its lake read completed (the one-hour memo itself is TIME-ENG-038).
- [x] **SYNC-MEMO-002**: If re-reading the climate record from the lake fails, then the system shall leave the slot unchanged and serve the last memoized record while its lake read completed less than 24 hours ago, marked as from the last good read so the Timing page shows a data note saying so, and return the error to the caller only when no such record exists.
