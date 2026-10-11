---
parent: high-level-design
prefix: WX
---

# Weather Data

## Problem

Every agronomic answer in TurfOps starts from measured conditions, and they arrive from three
places with different grains, units, lags and failure modes: the NOAA USCRN station
PA Avondale (WBANNO 3761) in the owner's weather lake, the Home Assistant patio sensor, and the
OpenWeatherMap 5-day / 3-hourly forecast. Five feature components and three read-only views
need these conditions. If each opened its own reads, they would disagree about what "now",
"today" and "the last 7 days" mean, and the station's lag and probe dropouts would be handled
differently on every page.

## Approach

This sub-HLD groups the readers, the shapes they publish, and the views built directly on
them. Three ideas hold across all of its children.

### The station is the record

Soil temperature and moisture, rain, freeze dates, GDD history and humidity history come from
the station's measured record, gaps and all. Modeled or gridded weather never stands in for it.
The patio sensor supplies only "ambient right now", and the forecast supplies only the future.

### The lake is read in place

The lake's silver (hourly) and gold (daily) parquet files are read where they sit, with
embedded DuckDB, on every request that needs them. Nothing here copies or caches raw weather.
Feature components consume the published query contracts rather than writing their own lake
SQL, so one change to a contract reaches every consumer.

### Lag is carried as data

The lake trails real time by about a day, the newest local day is usually partial, and the
forecast starts and ends mid-day. Contracts expose this instead of hiding it: the observation
stamp (`soil_observed_at`), per-day hour coverage (`hours_covered`), partial-day flags and
`full_days()`. Consumers decide how to disclose it. Missing values stay `None` all the way to
the UI, and sentinels are dropped rather than averaged in.

### One clock

"Today", every date gate and every year boundary use the server's local date
(`chrono::Local`, set by `TZ` to the lawn's timezone). Lake timestamps are UTC; observed hours
keep the lake's own day key, `obs_date_local` (the station's civil date in America/New_York,
daylight-saving aware), which is the same calendar `TZ` gives, so the lake is never re-bucketed.
Forecast slots are bucketed into those same local days. Each child flags the places where its
code still uses another clock.

## Children

| Child | Prefix | Owns |
|---|---|---|
| [Weather sources](weather-sources/weather-sources-design.md) | `WX-SRC` | The lake query contracts (silver and gold), lake access mechanics, the Home Assistant and OpenWeatherMap clients, `EnvironmentalReading` / `EnvironmentalSummary`, `WeatherForecast` with local-date bucketing and the day-level helpers (`full_days()` and friends) |
| [Soil-temperature outlook](soil-temp-outlook/soil-temp-outlook-design.md) | `WX-OUTLOOK` | The 10 cm air → soil regression, the direction-aware 10 cm thresholds, `GET /soil-temp-forecast`, `PredictionChart`, the dashboard `SoilTempForecastWidget`, and the `fit_model` / `predict_soil_temps` contract reused by timing windows |
| [Environmental views](environmental-views/environmental-views-design.md) | `WX-VIEW` | The Environmental page, `GET /historical`, the rendering of `/environmental`, the gauges and `TrendChart`, and the lake's connection-probe contract |

Weather sources is the foundation: the other two children consume its contracts exactly as
outside components do. The outlook and the views share no code with each other except that the
Environmental page hosts the outlook's chart.

## Boundaries to Consumers

| Component | Consumes | Owned there, not here |
|---|---|---|
| Data sync | `fetch_summary`, `fetch_gdd_ytd`, the HA and OWM fetches, each source's connection probe | *When* to read (5 min sensor / 30 min forecast staleness), the 72 h soil cut-off, the 6 h forecast reuse, failure retry policy, merging into the cached `EnvironmentalSummary`, and `/health` itself |
| Timing windows | `fetch_climate_record`, `fit_model` / `predict_soil_temps`, `WeatherForecast` | Windows, boundaries, climatology, the 5 cm outlook's series, gap interpolation and anchoring, the 1 h climate-record memo |
| Disease risk | `fetch_daily_disease_inputs`, `ForecastPoint`s | Which days are usable, the dawn and afternoon hour sets, the forecast-day series, every model |
| Agronomic rules | `EnvironmentalSummary` (incl. `gdd_base50_ytd`), the forecast helpers, `fetch_daily_gdd` behind the GDD endpoint | Rule thresholds and GDD accumulation |
| Seasonal plan | `fetch_daily_soil_temp_averages` | Crossing analysis and its Postgres cache |
| Landscape plants | `rain_expected_within`, the 7-day rain total, soil moisture | The watering hold-back rule |
| MCP server | `/soil-temp-forecast` as `soil_temp_outlook`; `fetch_daily_disease_inputs` and `fetch_climate_record` in `weather_history`; connection status in `lawn_snapshot` | Tool shapes and trimming |
| Web UI | The views' pages and widgets | Shell, routing, API client, shared styles and the status-palette conventions |

## Key Design Decisions

Decisions specific to one child live in that child's Decisions table.

| Decision | Chosen | Alternatives Considered | Rationale |
|---|---|---|---|
| How the lake is read | Embedded DuckDB over the parquet files on a read-only mount | A weather HTTP API; a Postgres copy; the MCP gateway | Local parquet reads are fast and always current. A copy goes stale, an API is one more service to run, and the gateway is an agent tool, not an app dependency (HLD; owner's lake-migration decision). |
| Raw weather caching | None. The views and contracts read silver and gold directly, so air and RH history is the station's, not the patio sensor's | An `environmental_cache` table of combined readings | Only expensive derived results are cached (HLD). Owner decision: the patio sensor has no history to keep. |
| Modeled weather as observed history | Never. The station is the record for soil, freeze dates, GDD history and humidity | Open-Meteo / ERA5 as a replacement or gap-free source | Measured against 14 years at the station: crossings 4–10 days early, first freeze 17.5 days late, moisture +0.13 wetter, humid hours under-read (owner's Open-Meteo comparison). |
| Clock for "today" and day buckets | The server's local date (`chrono::Local`, `TZ` = the lawn's timezone) for every date gate, year boundary and forecast day bucket; the lake's `obs_date_local` (the same Eastern calendar) for observed days | The UTC date; a station-local "today" from the lake's UTC offset; the forecast location's offset from OWM; re-bucketing lake hours by `TZ` | HLD decision: every agronomic date is a lawn-local calendar day, and one configured clock gives every component the same day. The UTC date rolls over hours early every evening. |
| Precipitation units | Millimetres in every backend contract; only the UI converts to inches (`frontend/src/utils/units.ts`) | Inches in the API | One unit across the lake, the forecast, the models and the rules. [inferred] |
| Shape of this node | A sub-HLD over three leaves | One leaf; the outlook as a flat sibling | Sources are consumed by five components and would be the only part touched by a forecast-provider swap; the outlook is a model with its own thresholds and a reuse contract; the views are UI. All three share the station-as-record, read-in-place and lag-as-data intent held here. |

## References

- `docs/high-level-design.md`: tenets, the "Weather data access" component, the in-place lake
  and "today" decisions.
- `docs/agronomy-methods.md`: data and staleness rules, thresholds (served over MCP).
- `docs/timing-windows.md`: the 5 cm outlook built on this node's regression.
- `docs/plans/open-meteo-forecast.md`, `docs/plans/uscrn-soil-dropout-investigation.md`
  (both parked).
- `docs/weather-data-api-spec.md`: draft HTTP API over the lake (not adopted).
- Sibling designs: `docs/intent/data-sync/`, `timing-windows/`, `disease-risk/`,
  `agronomic-rules/`, `seasonal-plan/`, `landscape-plants/`, `mcp-server/`, `web-ui/`.
- NOAA USCRN hourly02 product, station PA Avondale 2 N (WBANNO 3761).
