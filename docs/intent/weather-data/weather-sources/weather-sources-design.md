---
parent: weather-data
prefix: WX-SRC
---

# Weather Sources

## Context and Design Philosophy

This leaf owns the three readers that bring measured and forecast conditions into TurfOps and
the shapes they publish: the weather lake (NOAA USCRN PA Avondale, WBANNO 3761) read in place,
the Home Assistant patio sensor, and the OpenWeatherMap 5-day / 3-hourly forecast. It defines
*what* each read returns. Data sync decides *when* to call them and merges their output; the
feature components that consume a contract own what they do with it (see the parent's
boundary table).

Principles, from the parent: the station is the record, so every observed contract comes from
the lake; contracts carry the lag (stamps, hour coverage, partial days) rather than hiding it;
missing values stay `None` and sentinels are dropped; and every date is a lawn-local day on the
app's one clock.

## Sources

| Source | Transport | Grain / units at the source | Supplies | Code |
|---|---|---|---|---|
| Lake **silver** `silver_weather.parquet` | Embedded DuckDB `read_parquet`, filtered `CAST(wbanno AS INTEGER) = station` | Hourly; °C, mm, % RH, fractional moisture; `obs_ts_utc`, `obs_ts_local`, `obs_date_local` | Live reading, 7-day summary, trend, `/historical`, daily disease inputs, daily 5 cm soil means | `backend/src/datasources/weather.rs`, `weather/latest.rs`, `weather/disease.rs`, `weather/climatology.rs` |
| Lake **gold** `daily_weather.parquet` | Embedded DuckDB, **no station filter** (single-station gold) | Daily; °F; `gdd50`; `hours_observed` | 10 cm daily means, air/soil pairs, GDD rows, freeze-date air minima | same |
| Home Assistant | `GET {HA_URL}/api/states/{entity}` with a bearer token; 5 s connect / 10 s request timeouts | Instantaneous entity state string | `ambient_temp_f`, `humidity_percent` | `backend/src/datasources/homeassistant.rs` |
| OpenWeatherMap | `GET /data/2.5/forecast?lat&lon&units=imperial`; 5 s / 15 s timeouts | 40 points at 3 h steps; °F, mph, mm per 3 h, `pop` 0–1; `city.timezone` in seconds | `WeatherForecast` | `backend/src/datasources/openweathermap.rs` |

Configuration (`backend/src/config.rs`): `DATALAKE_ROOT` (default `/data`) derives
`{root}/silver/weather/silver_weather.parquet` and `{root}/gold/weather/daily_weather.parquet`.
`WEATHER_SILVER_PATH` and `WEATHER_GOLD_PATH` override them. `NOAA_STATION_WBANNO` (default
3761; an unparsable value logs a warning and falls back to 3761). Home Assistant: `HA_URL`,
`HA_TOKEN` (empty means not configured), `HA_TEMPERATURE_ENTITY`, `HA_HUMIDITY_ENTITY`,
`HA_TEMPERATURE_UNIT` (`celsius` converts, anything else means °F). OpenWeatherMap:
`OWM_API_KEY` (blank means not configured), `OWM_LATITUDE` / `OWM_LONGITUDE` (default 0),
`OWM_ENABLED` (default true). Data sync always builds the lake client and builds the other two
only when they are configured.

## Lake Access Mechanics

- **One connection per query.** DuckDB connections are not `Sync`, so `WeatherLakeClient::run`
  opens a fresh in-memory connection inside `spawn_blocking` and scans the parquet directly.
  Nothing is pooled or cached at this layer. A task join failure becomes
  `DataSourceUnavailable`, and DuckDB errors become `TurfOpsError::DataLake` (HTTP 500 with a
  generic body).
- **Paths** are trusted config, interpolated with single quotes escaped (`parquet()`). All
  user-influenced values (station, dates, timestamps) are bound parameters.
- **Units.** Silver °C is converted to °F at ingestion (`celsius_to_fahrenheit`,
  `backend/src/models/environmental.rs`). Gold is already °F. Precipitation stays in mm.
- **Sentinel normalization** (applied in the app even though the lake cleans it too):

| Field | Kept when | Where |
|---|---|---|
| Soil / air temperature (°C) | `> -50` | `row_to_reading`, `fetch_latest` SQL, climate-record SQL |
| Soil moisture, RH, precipitation | `>= 0` | `row_to_reading` (RH `> 0` in the disease query) |

### Time, lag and day buckets

The lake loads through local midnight, so its newest data trails real time by about a day and
the most recent local day is usually partial. Lake timestamps are UTC. Every daily contract
buckets hours into lawn-local calendar days, and every "today" or year boundary is the lawn's
local date from the app's `Clock` (`backend/src/clock.rs`, resolved once from `TZ`; see the
deployment design). The lake contracts publish no "today" of their own: a caller that needs one
passes the `Clock`'s date in (`fetch_gdd_ytd(today)`, the start dates of the other reads).

The one lag contract is the **observation stamp**: `fetch_latest` stamps the reading with the
newest silver `obs_ts_utc`, which data sync publishes as `soil_observed_at`.

Daily contracts bucket by the lake's own `obs_date_local`: the station's civil date in
America/New_York, daylight-saving aware (offsets −300 / −240). That is the calendar `TZ` gives
for this lawn, so it is the intended day key and the lake is not re-bucketed.

## Lake Query Contracts

| Method | Layer | Returns | Gates | Order | Consumer |
|---|---|---|---|---|---|
| `fetch_latest` | silver | `Option<EnvironmentalReading>`: **each column's newest non-null, non-sentinel value** within 24 h of the newest row, stamped with the newest row's time | station; `HAVING count(*) > 0` | n/a | data sync (via `fetch_summary`) |
| `fetch_range(start, end)` | silver | Hourly `EnvironmentalReading`s in [start, end] | station | **newest first** | `fetch_summary`, `/historical` |
| `fetch_summary` | silver | `EnvironmentalSummary`: `current` from `fetch_latest`; 7-day means of 10 cm soil, air and RH; 7-day precipitation total; soil trend | the 7 × 24 h before now | n/a | data sync |
| `fetch_daily_soil_temp_averages` | gold | `(date, soil_temp_10_f_mean)` | `hours_observed >= 12`, non-null | asc | seasonal plan |
| `fetch_daily_paired_averages` | gold | `(date, air_temp_avg_f, soil_temp_10_f_mean)` | `hours_observed >= 12`, both non-null | asc | soil-temperature outlook |
| `fetch_daily_gdd(start, end)` | gold | `(date, max_f, min_f, gdd50)`. Null max/min/gdd50 read as 0.0 (display only) | `gdd50 IS NOT NULL` | asc | `/historical`, the GDD endpoint (agronomic rules) |
| `fetch_gdd_ytd(today)` | gold | Sum of `gdd50` from Jan 1 of `today`'s year through `today` (the lawn's local date). `None` if there are no rows | as above | n/a | data sync → `gdd_base50_ytd` |
| `fetch_daily_disease_inputs(start)` | silver | `LakeDailyWeather`: one `DailyWeather` per local day | hours with `air_temp_c` non-null and `rh_pct > 0` | asc | disease risk, MCP server (`weather_history`) |
| `fetch_climate_record(start)` | silver + gold | `ClimateRecord`: `ClimateDay {soil_temp_5_f, air_min_f, air_avg_f, gdd50}` per local day | 5 cm: ≥ 12 valid hourly values per local day. Gold: `hours_observed >= 12` | asc | timing windows (memoized there), MCP server |

The lake's connection probe (`test_connection`) is described by the environmental-views leaf.

Contract details consumers rely on:

- **Per-sensor live reading** (`weather/latest.rs`). The station's soil probes drop individual
  hours, mostly daytime ones, so the newest row often exists with empty soil columns. Each of
  the 13 columns is `arg_max(col, obs_ts_utc)` filtered to non-null values `> -50`, over the
  24 h ending at the newest row. A column with no good value in that span is `None`. The stamp
  is the newest row's time, not the age of each value.
- **Trend** (`calculate_trend`). Needs ≥ 48 rows. It compares the mean 10 cm temperature of the
  first 24 rows with the next 24 (newest first): more than +2 °F is `Rising`, less than −2 °F
  is `Falling`, otherwise `Stable`. Fewer rows, or no 10 cm values in either half, gives
  `Unknown`.
- **Disease inputs** (`weather/disease.rs`). Per local day: `hours_covered` (usable hours),
  air mean, min (`air_temp_min_c` falling back to `air_temp_c`), max (likewise), RH mean, hours
  with RH ≥ 90, leaf-wetness hours (RH ≥ 90 **or** precip > 0), precipitation sum, maximum
  Magnus dew point, and `covers_dawn` / `covers_afternoon`. Those two flags mean at least one
  hour falls inside `DAWN_HOURS` or `AFTERNOON_HOURS`, constants owned by disease risk.
  `is_forecast` is false. Values stay in °C. The last day is usually partial, and deciding
  whether a day is usable is left to disease risk.
- **Climate record** (`weather/climatology.rs`). Soil is the mean of hourly `soil_temp_5`
  (°C → °F) per local day. Air min/mean and `gdd50` come from gold `day`. Days are merged by
  date, so a day present in only one layer appears with the other fields `None`.

## Home Assistant Reader

`HomeAssistantClient::fetch_current` reads the temperature entity, then the humidity entity.
Each one is independent. A non-numeric state (for example `unavailable`) or a failed request
logs a warning and leaves that field `None`, and the call itself still succeeds. Temperature is
converted when `HA_TEMPERATURE_UNIT=celsius`. The reading is stamped with the server clock and
has `source = HomeAssistant`. HA provides only the latest state, with no history, so ambient
history comes from the station. `test_connection` returns whether `GET /api/` succeeds.

## OpenWeatherMap Forecast

`fetch_forecast` returns a `WeatherForecast` (`backend/src/models/forecast.rs`):

- `hourly: Vec<ForecastPoint>`: one point per 3 h slot. Precipitation is rain plus snow in mm
  for the slot. `weather_condition` maps OWM ids as follows: 2xx Thunderstorm, 3xx Drizzle,
  5xx Rain, 6xx Snow, 701 Mist, 741 Fog, 800 Clear, 801–804 Clouds, anything else Other.
- `daily_summary: Vec<DailyForecast>`: points are grouped by **lawn-local date**. Grouping by
  UTC date would put an evening high and the next dawn low in different days. Each day has its
  high and low (max/min of slot temps), mean RH, total precipitation, maximum `pop`, the most
  frequent condition, mean wind, maximum gust, and `hours_covered = 3 × points`. The local date
  is `clock.local_date(point.timestamp)` (`aggregate_daily`, `openweathermap.rs`): the client
  holds the app's `Clock`, so each point lands on its own `TZ` date, daylight-saving aware.
  OWM's `city.timezone` is carried as `location.utc_offset_seconds` for information only and
  dates nothing.

Day-level helpers, a contract consumed by rules, timing windows, the soil-temperature outlook
and landscape plants:

| Helper | Semantics |
|---|---|
| `DailyForecast::is_full_day` | `hours_covered >= 18` (≥ 6 slots) |
| `full_days()` | Only full days. **Anything that needs a daily mean or judges a whole day must use this**, because the feed starts and ends mid-day. |
| `next_hours(n)` | Points with `timestamp <= now + n h`. Points already past are not dropped, so the first slot (current weather) counts. |
| `next_days(today, n)` | Daily summaries with `date <= today + n`, partial days included. `today` is the caller's lawn-local date; the forecast keeps no clock of its own |
| `rain_expected_within(h, mm)` | `Some` when total precipitation in `next_hours(h)` ≥ `mm` **or** any slot's `pop` ≥ 0.5 |
| `max_temp_next_days(today, n)` | Highest daily high in `next_days(today, n)` |

Errors: a transport, status or parse failure returns `DataSourceUnavailable`. The request URL
carries the key as `appid=` in its query string, and reqwest's error text includes the URL, so
every reqwest error from `fetch_forecast` and `test_connection` goes through `request_error`,
which formats `e.without_url()`, and the JSON-decode error is formatted the same way. No error
text, log line or response can therefore carry the key. A non-success status includes the
response body in the message (Deferred 1). Data sync logs the error and keeps the previous
forecast within its reuse limit.

## Environmental Data Model

`EnvironmentalReading` holds soil temperature at 5/10/20/50/100 cm (°F), soil moisture at the
same depths (fraction 0–1), `ambient_temp_f`, `humidity_percent`, `precipitation_mm` (the
newest hourly value), `timestamp` and `source`. `primary_soil_moisture()` uses 10 cm, then
5 cm, then 20 cm.

`EnvironmentalSummary` is the published shape of current conditions:

| Field | Filled from (data sync merges) |
|---|---|
| `current` soil temp / moisture / precipitation | lake `fetch_latest`. Omitted when the stamp is more than 72 h old (data sync rule) |
| `current.ambient_temp_f`, `current.humidity_percent` | Home Assistant only |
| `soil_temp_7day_avg_f` (10 cm), `ambient_temp_7day_avg_f`, `humidity_7day_avg`, `precipitation_7day_total_mm`, `soil_temp_trend` | lake silver, last 7 days |
| `soil_observed_at` | the lake reading's stamp (omitted from JSON when `None`) |
| `last_updated` | when data sync last refreshed. This is the app's clock, not the station's |
| `forecast` | OpenWeatherMap (omitted when `None`) |
| `gdd_base50_ytd` | `fetch_gdd_ytd` |

`DataSource` is the shared label vocabulary for data points across the feed. The lake's variant
is `SoilData`, labelled "NOAA USCRN", and the patio sensor's is "Patio Sensor".

## Failure and Degradation

| Failure | Effect at the source contracts |
|---|---|
| Lake unreadable | `fetch_*` returns `DataLake` or `DataSourceUnavailable`. Data sync keeps its last good summary |
| Lake stalled (no new rows) | The live reading carries the old stamp, and data sync stops using it as current after 72 h. Every daily contract simply ends earlier |
| Individual soil hours missing | The live reading falls back per sensor within 24 h. Daily 5 cm means need ≥ 12 valid hours. Gold-backed contracts gate on `hours_observed`, which counts rows rather than soil hours |
| Home Assistant down or entity unavailable | That field is `None`. The 7-day means are unaffected because they come from the station |
| OpenWeatherMap down | `DataSourceUnavailable`. Consumers lose the forecast once data sync's reuse window expires |

## Decisions & Alternatives

| Decision | Chosen | Alternatives Considered | Rationale |
|---|---|---|---|
| Connection lifetime | A fresh in-memory DuckDB connection per query inside `spawn_blocking` | A pooled or shared connection | DuckDB connections are not `Sync`. Queries are local and cheap, so per-query setup costs little. [inferred] |
| Which layer backs what | Silver for the live reading, 7-day summary, trend, `/historical` and disease inputs. Gold for the 10 cm regression, seasonal plan, GDD and freeze minima | All from silver, aggregated in-app | Gold already has daily °F means and `gdd50`, and silver keeps the hourly detail the live views need. Owner's lake-migration decision. |
| GDD source | Sum gold `gdd50` in Rust. No `gdd_daily` cache | Recompute from max/min and cache it | Gold's `gdd50` was verified identical to the app's own base-50 formula. A cache would only duplicate it (owner decision). |
| Live reading | Each sensor's newest non-null value within 24 h of the newest row | The newest row as-is | The station's probes drop (mostly daytime) hours. The newest row often has empty soil columns, which blanks the gauges and silences every soil rule each afternoon (owner, `docs/plans/uscrn-soil-dropout-investigation.md`; code comment in `weather/latest.rs`). |
| Sentinel handling | Drop temperatures ≤ −50 °C and negative moisture, RH and precipitation in the app as well | Trust the lake's cleaning | One surviving −9999 hour would move a 7-day mean by about 100 °F (comment in `weather.rs`). [inferred] |
| Forecast day buckets | Lawn-local dates from the app's `Clock` (`TZ`), with `hours_covered` and `full_days()` for whole-day judgments | UTC dates; OWM's `city.timezone` offset; treat every bucket as a full day | UTC dates split an afternoon high from the next dawn low, and the feed starts and ends mid-day (`docs/agronomy-methods.md`). One clock for every date (parent decision): OWM's offset is a fixed number for the fetch, so it misplaces points across a daylight-saving change, and it is a second clock beside `TZ`. |
| API key in errors | Strip the URL from every reqwest error (`without_url()`) inside the client | Rely on callers not to log or return the error; redact the text after formatting | The key must travel in the query string, so the client is the one place that can guarantee no caller leaks it. |
| Full-day cut-off | ≥ 18 of 24 h (6 slots) | 24 h only; any coverage | Allows a one-slot shortfall while still excluding evening-only buckets. [inferred] |
| Forecast provider | OpenWeatherMap 5-day / 3-hourly | Open-Meteo hourly, no key, 7–16 days | Parked by the owner: live with the current implementation and adopt only when a trigger in `docs/plans/open-meteo-forecast.md` is observed. |
| Current ambient source | Home Assistant patio sensor for "now". Station air for the 7-day means and history | Station air for everything | The patio sensor is at the yard and current. The station is about 15 mi away and a day behind. HA has no history, so history is the station's. [inferred] |
| Trend with no data | Unknown | Stable (the type's default) | Withhold rather than invent: with no rows there is no evidence the soil is steady. |
| Forecast without coordinates | Not enabled, with a log line | Default to 0,0 | A forecast for the wrong place is worse than none; every forecast consumer already degrades without one. |
| Gold station filter | Gold is queried without a `wbanno` filter | Filter gold by station | The lake is single-station per deployment (`docs/weather-data-api-spec.md` §2.2). |

## Open Questions & Future Decisions

### Resolved

1. ✅ The live reading is per sensor, which absorbs the station's daytime probe dropout.
2. ✅ Forecast days are lawn-local, and whole-day judgments use `full_days()`.

### Deferred

1. **Divergence: upstream response bodies in forecast errors** (WX-SRC-OWM-006). A non-success
   status formats the response body into the `DataSourceUnavailable` message
   (`backend/src/datasources/openweathermap.rs`, `fetch_forecast`). Intent: the message is plain
   language naming the forecast as unavailable, and upstream bodies go to the log only.
2. **Divergence: forecast at 0,0** (WX-SRC-CFG-003). With only `OWM_API_KEY` set, latitude and
   longitude silently default to 0 (`backend/src/config.rs:241-242`). Intent: the forecast is not
   enabled, and the log says why.
3. **Open-Meteo forecast swap (parked).** Adopt only when a trigger in
   `docs/plans/open-meteo-forecast.md` appears: a recurring "most recent complete day" banner on
   Disease Risk, 3-hourly quantization visibly biting, a 5-day horizon too short for the Timing
   outlook, or OWM unreliability. Two decisions stay open: remove OWM or keep it as a fallback,
   and whether to use `past_days` to fill the same-day gap. If it lands, `hours_covered =
   3 × points` and the disease 3 h step must become data-driven, and Open-Meteo soil variables
   must not be requested. A failure cool-down for the forecast is wanted independently of the
   swap; it belongs to data sync (its Deferred 3).
4. **USCRN soil dropout (lake-side).** Investigation in
   `docs/plans/uscrn-soil-dropout-investigation.md`. Preferred gap-fill order: USCRN 5-minute
   `subhourly01` (measured 5 cm), then the app's air → soil regression, then Open-Meteo with a
   per-month bias correction. If gold gains per-sensor hour counts, the gold-backed contracts
   should gate on them instead of `hours_observed`.
5. **Weather HTTP API draft** (`docs/weather-data-api-spec.md`): not adopted, because the HLD
   chose in-place reads. Its operation #1 still describes the live reading as "newest row
   LIMIT 1", which is out of date.
6. **Divergence: summary with no silver rows** (WX-SRC-LAKE-013). `fetch_summary` returns early with
   `soil_temp_trend = Stable` (the `Trend` default, `backend/src/models/environmental.rs:126`),
   not `Unknown` (`weather.rs:110-112`). The 7-day window is the 168 h before now, so with the
   lake lag it holds about 6 days of data.
7. **Per-sensor values older than the stamp.** `fetch_latest` stamps the reading with
   `max(obs_ts_utc)` of any column (`weather/latest.rs:45-49`), so a soil value can be up to
   24 h older than `soil_observed_at`. The 72 h rule and the dashboard banner judge only the
   stamp.
8. **Home Assistant staleness.** The reading is stamped with the server clock
    (`backend/src/datasources/homeassistant.rs:80`) and the entity's `last_changed` is ignored,
    so a stuck sensor reads as current.
9. **Gold has no station filter.** This silently breaks if gold ever holds more than one
    station or `NOAA_STATION_WBANNO` is changed.
10. **Legacy naming.** `DataSource::SoilData` (labelled "NOAA USCRN") names the lake.
11. **Technical debt.** `backend/src/datasources/weather.rs` is 381 lines with its tests
    (limit 300). `aggregate_day` in `openweathermap.rs` is over 50 lines (~55). No tests cover
    `calculate_trend` / `fetch_summary`, `fetch_range` or the Home Assistant client.

## References

- Parent: `docs/intent/weather-data/weather-data-design.md` (shared intent, consumer boundaries).
- `docs/agronomy-methods.md`: data and staleness rules.
- `docs/plans/open-meteo-forecast.md`, `docs/plans/uscrn-soil-dropout-investigation.md`.
- `docs/weather-data-api-spec.md`: lake schema notes and the unadopted HTTP API draft.
- `docs/intent/data-sync/`: refresh timing, merging, failure policy.
- Code: `backend/src/datasources/{weather.rs, weather/latest.rs, weather/disease.rs,
  weather/climatology.rs, homeassistant.rs, openweathermap.rs, openweathermap/tests.rs}`,
  `backend/src/models/{environmental,forecast}.rs`, `backend/src/config.rs`.
