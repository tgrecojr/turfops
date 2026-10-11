# NOAA USCRN Weather Data API — Specification

**Status:** Draft for implementation
**Audience:** A team/agent building a new, standalone HTTP service in a separate codebase.
**Goal:** Put a read-only REST API in front of a NOAA USCRN weather **data lake** (Dagster
medallion parquet files), replacing direct in-process file reads by consuming applications.

This document is self-contained. You do **not** need access to the consuming application's
source to implement it. It describes the underlying data, the exact access patterns in use
today, and the target API surface.

---

## 1. Background & context

### 1.1 What the data is

NOAA operates the **U.S. Climate Reference Network (USCRN)** — a network of research-grade
weather stations recording air temperature, precipitation, relative humidity, and (at most
stations) **soil temperature and soil moisture at five depths**. Observations are produced
hourly.

A **Dagster pipeline** ingests USCRN data and lands it in a **medallion-architecture data
lake** (parquet files on a filesystem):

- **Bronze** — raw ingested data (not consumed by the API).
- **Silver** — cleaned, deduplicated, one row per hourly observation, in **native units**
  (°C, mm, % RH, fractional soil moisture 0–1).
- **Gold** — daily pre-aggregated means, already converted to **°F**, with a precomputed
  growing-degree-day value (`gdd50`).

### 1.2 How it is accessed today (the thing we are replacing)

Consuming applications currently open an **embedded DuckDB** connection in-process and run
`read_parquet()` directly against the silver and gold files on a mounted filesystem. There
is no network service; the lake is a local mount.

Default file locations (derived from a `DATALAKE_ROOT`, default `/data`):

| Layer  | Default path                                  | Grain  | Units                       |
|--------|-----------------------------------------------|--------|-----------------------------|
| silver | `/data/silver/weather/silver_weather.parquet` | hourly | native (°C, mm, %, 0–1)     |
| gold   | `/data/gold/weather/daily_weather.parquet`    | daily  | °F (+ precomputed `gdd50`)  |

The new service will own these file reads (via DuckDB, Polars, DuckDB-over-HTTP, a query
engine, or a database load — implementer's choice) and expose the HTTP API below. Consumers
will switch from local DuckDB reads to HTTP calls.

### 1.3 Why an API

- Decouple consumers from the lake's physical layout and the DuckDB dependency.
- Centralize the station filter, quality gates, unit conventions, and null-normalization.
- Allow multiple consumers / languages without each re-implementing parquet access.

---

## 2. Source data schemas (parquet columns)

These are the exact columns the current consumer selects. The new service reads these and
maps them to the API response schemas in §4.

### 2.1 Silver — `silver_weather.parquet` (hourly, native units)

| Column            | Type             | Notes                                                        |
|-------------------|------------------|--------------------------------------------------------------|
| `obs_ts_utc`      | TIMESTAMP (UTC)  | Observation time. Primary sort key.                          |
| `wbanno`          | string/int       | USCRN station id (WBANNO). **Filter:** `CAST(wbanno AS INTEGER) = :station`. |
| `soil_temp_5`     | double (°C)      | Soil temp at 5 cm. Nullable.                                 |
| `soil_temp_10`    | double (°C)      | Soil temp at 10 cm. Nullable. **(primary series used by consumers)** |
| `soil_temp_20`    | double (°C)      | Soil temp at 20 cm. Nullable.                                |
| `soil_temp_50`    | double (°C)      | Soil temp at 50 cm. Nullable.                                |
| `soil_temp_100`   | double (°C)      | Soil temp at 100 cm. Nullable.                               |
| `soil_moisture_5` | double (0–1)     | Volumetric soil moisture at 5 cm. Negative = missing sentinel. |
| `soil_moisture_10`| double (0–1)     | At 10 cm. Negative = missing sentinel.                       |
| `soil_moisture_20`| double (0–1)     | At 20 cm. Negative = missing sentinel.                       |
| `soil_moisture_50`| double (0–1)     | At 50 cm. Negative = missing sentinel.                       |
| `soil_moisture_100`| double (0–1)    | At 100 cm. Negative = missing sentinel.                      |
| `air_temp_c`      | double (°C)      | Ambient air temp. Nullable.                                  |
| `rh_pct`          | double (%)       | Relative humidity. Negative = missing sentinel.              |
| `precip_mm`       | double (mm)      | Precipitation. Negative = missing sentinel.                  |
| `obs_ts_local`    | TIMESTAMP        | Station-local observation time (DST-aware). Used with `obs_ts_utc` to derive the UTC offset. *(disease-risk query, added 2026-09)* |
| `obs_date_local`  | DATE             | Station-local calendar date. Day-bucketing key for disease risk. *(added 2026-09)* |
| `air_temp_min_c`  | double (°C)      | Within-hour minimum air temp. Nullable; consumer falls back to `air_temp_c`. *(added 2026-09)* |
| `air_temp_max_c`  | double (°C)      | Within-hour maximum air temp. Nullable; consumer falls back to `air_temp_c`. *(added 2026-09)* |

The silver file also carries `solar_rad_wm2`, `surface_temp_c` (+ min/max) and `obs_date_utc`,
which no consumer reads today.

**Normalization rule (applied today, must be preserved):** any soil-moisture, humidity, or
precipitation value `< 0` is a missing-data sentinel and must be returned as `null`, not as
a negative number. Soil temps and air temp are passed through as-is (may be null).

### 2.2 Gold — `daily_weather.parquet` (daily, °F)

| Column                | Type        | Notes                                                       |
|-----------------------|-------------|-------------------------------------------------------------|
| `day`                 | DATE        | Calendar day. Sort key.                                     |
| `soil_temp_10_f_mean` | double (°F) | Daily mean soil temp at 10 cm. Nullable.                    |
| `air_temp_avg_f`      | double (°F) | Daily mean air temp. Nullable.                              |
| `air_temp_max_f`      | double (°F) | Daily max air temp. Nullable (can be null on sparse days).  |
| `air_temp_min_f`      | double (°F) | Daily min air temp. Nullable (can be null on sparse days).  |
| `gdd50`               | double      | Growing degree days, base 50°F, precomputed by the lake.    |
| `hours_observed`      | int         | Hours contributing to the daily aggregate. Quality gate.    |

> **`gdd50` definition:** the lake computes it as `max((air_temp_max_f + air_temp_min_f)/2 - 50, 0)`
> per day. It has been verified identical to the consumer's own formula. The API exposes it
> as-is; year-to-date totals are sums of `gdd50`.

> **Note:** the gold table is not station-partitioned in the current consumer's queries (it
> queries gold with no `wbanno` filter, since the lake is single-station per deployment). If
> your gold parquet carries a station column, add the same `:station` filter for parity. The
> API contract below accepts a `station` param on gold endpoints for forward-compatibility;
> if gold is single-station, you may ignore it or validate it matches the configured station.

---

## 3. Exact current access patterns (the behaviors to preserve)

The new API must reproduce these ten operations. Operations 1–8 map to an endpoint in §4;
operations 9 (disease risk) and 10 (timing windows), both added 2026-09, do not yet — see
§3.3 and §3.4.

| # | Operation                       | Layer  | Query semantics                                                                                 | Ordering   |
|---|---------------------------------|--------|-------------------------------------------------------------------------------------------------|------------|
| 1 | Latest observation              | silver | newest row for station: `WHERE wbanno=:s ORDER BY obs_ts_utc DESC LIMIT 1`                       | n/a        |
| 2 | Observations in range           | silver | `WHERE wbanno=:s AND obs_ts_utc BETWEEN :start AND :end`                                         | **DESC** (newest-first) |
| 3 | 7-day summary + trend (derived) | silver | latest row + 7-day window of hourly rows → means + trend (see §3.1)                              | n/a        |
| 4 | Daily soil-temp averages        | gold   | `WHERE day BETWEEN :start AND :end AND hours_observed>=12 AND soil_temp_10_f_mean IS NOT NULL`   | **ASC**    |
| 5 | Daily paired air+soil averages  | gold   | as #4 plus `air_temp_avg_f IS NOT NULL` (returns day, air_temp_avg_f, soil_temp_10_f_mean)       | **ASC**    |
| 6 | Daily GDD rows                  | gold   | `WHERE day BETWEEN :start AND :end AND gdd50 IS NOT NULL` (returns day, max, min, gdd50)         | **ASC**    |
| 7 | GDD year-to-date (derived)      | gold   | sum of `gdd50` from Jan 1 of year to today                                                       | n/a        |
| 8 | Health / connectivity probe     | gold   | `SELECT 1 FROM gold LIMIT 1` returns a row                                                       | n/a        |
| 9 | Daily disease-model inputs      | silver | hourly rows aggregated per **local** day from `:start` (see §3.3), plus the station's current UTC offset | **ASC**    |
| 10 | Multi-year daily climate record | silver + gold | daily mean 5 cm soil temp per local day (silver) merged with daily air min/mean + `gdd50` (gold) from `:start`, ~15 years (see §3.4) | **ASC**    |

Operations 4, 5, and 6 all select from the same gold rows with different column projections
and null-guards — in the API they are **one endpoint** (`/daily`) with field selection (§4.3).

### 3.1 Derived: 7-day summary + soil-temp trend

Computed from hourly silver rows over the last 7 days (newest-first):

- `soil_temp_10_avg_f` — mean of non-null `soil_temp_10` (converted °C→°F).
- `ambient_temp_avg_f` — mean of non-null `air_temp_c` (→°F).
- `humidity_avg` — mean of non-null, non-negative `rh_pct`.
- `precip_total_mm` — sum of non-negative `precip_mm`.
- `soil_temp_trend` — compare the **last 24 hourly readings** vs the **prior 24**:
  - requires ≥ 48 readings total, else `unknown`;
  - `diff = mean(recent24) − mean(prior24)` of `soil_temp_10` in °F;
  - `diff > +2.0°F` → `rising`; `diff < −2.0°F` → `falling`; otherwise `stable`;
  - if either window has no non-null soil temp → `unknown`.

### 3.2 Unit conversion boundary

Today the consumer converts °C→°F itself after reading silver. **Decision for this spec:**
the **silver endpoints return native units** (°C/mm/%/0–1) so existing conversion logic is
unaffected; **gold endpoints return °F** (the lake already stores °F). The **derived summary**
(§3.1) returns °F because it is a higher-level convenience. `°F = °C × 9/5 + 32`.

### 3.3 Daily disease-model inputs (operation 9)

Added 2026-09 for the per-disease risk models (`WeatherLakeClient::fetch_daily_disease_inputs`
in `backend/src/datasources/weather/disease.rs`). The gold layer cannot serve this: it has no
humidity-duration, leaf-wetness, or dew-point fields, and the models need local-day bucketing.

Rows are filtered to `wbanno=:s AND obs_date_local >= :start AND air_temp_c IS NOT NULL AND
rh_pct > 0`, grouped by `obs_date_local`, ascending. Per day (all native units, °C):

| Output               | Derivation                                                              |
|----------------------|-------------------------------------------------------------------------|
| `hours_covered`      | `count(*)` of usable hours (24 = complete; the newest day is usually partial) |
| `temp_mean_c`        | `avg(air_temp_c)`                                                        |
| `temp_min_c`         | `min(coalesce(air_temp_min_c, air_temp_c))`                              |
| `temp_max_c`         | `max(coalesce(air_temp_max_c, air_temp_c))`                              |
| `rh_mean`            | `avg(rh_pct)`                                                            |
| `hours_rh90`         | hours with `rh_pct >= 90`                                                |
| `leaf_wetness_hours` | hours with `rh_pct >= 90 OR precip_mm > 0` (estimate)                    |
| `precip_mm`          | `sum(precip_mm)`                                                         |
| `dew_point_max_c`    | max hourly Magnus dew point from `air_temp_c` + `rh_pct`                 |

A second scalar query returns `date_diff('minute', obs_ts_utc, obs_ts_local)` for the newest
row, so the consumer can bucket forecast points into the same local days.

**API implication (open):** no endpoint in §4 covers this. Options: (a) a new
`GET /weather/daily/disease-inputs?start=` returning the table above plus `utc_offset_minutes`;
(b) expose `air_temp_min_c`/`air_temp_max_c`/local timestamps on `/weather/observations` and let
the consumer aggregate ~300 hourly rows client-side. The OpenAPI file has **not** been updated
for this.

### 3.4 Multi-year daily climate record (operation 10)

Added 2026-09 for the seeding / pre-emergent timing windows
(`WeatherLakeClient::fetch_climate_days` in `backend/src/datasources/weather/climatology.rs`;
method in `docs/timing-windows.md`). Two queries from a single `:start` (Jan 1, 15 years
back), merged by date into one ascending series where any field may be null:

| Output          | Layer  | Derivation                                                                 |
|-----------------|--------|----------------------------------------------------------------------------|
| `soil_temp_5_c` | silver | `avg(soil_temp_5)` grouped by `obs_date_local`, `WHERE wbanno=:s AND soil_temp_5 IS NOT NULL AND soil_temp_5 > -50`, `HAVING count(*) >= 12` (consumer converts to °F) |
| `air_temp_min_f`| gold   | as stored, `WHERE hours_observed >= 12`                                     |
| `air_temp_avg_f`| gold   | as stored                                                                  |
| `gdd50`         | gold   | as stored                                                                  |

Silver is used for soil because gold only aggregates the 10 cm probe. The `> -50` guard
drops any raw USCRN `-9999` sentinel that survives cleaning. The consumer memoizes the
result for an hour, so this is a low-frequency, large-span read (~5,500 rows out).

**API implication (open):** no endpoint in §4 covers this, and the 400-day max span in §4.0
would reject it. Options: (a) add `soil_temp_5_f_mean` (and ideally the other depths) to the
gold layer and lift the span cap for `/daily` when few fields are selected; (b) a dedicated
`GET /weather/daily/climate?start=`. The OpenAPI file has **not** been updated for this.

---

## 4. API specification

### 4.0 Conventions

- **Base path:** `/api/v1/weather`
- **Format:** JSON. `Content-Type: application/json`.
- **Auth:** internal service — bearer token or mTLS. (Implementer's choice; not security-sensitive data, but the endpoint should not be open to the public internet.)
- **Time:** request/response timestamps are ISO-8601 **UTC** (`2026-06-27T14:00:00Z`); dates are `YYYY-MM-DD`.
- **Station:** `station` query param = WBANNO integer. **Default `3761`** (USCRN PA Avondale) if your deployment is single-station; otherwise required.
- **Ranges:** `start`/`end` inclusive. Reject ranges where `end < start` (400). Enforce a max span (suggest 400 days) and return 400 if exceeded.
- **Errors:** `application/problem+json`:
  ```json
  { "type": "about:blank", "title": "Invalid range",
    "status": 400, "detail": "end must be >= start" }
  ```
  - `400` — bad params (range, unknown field, bad date).
  - `404` — unknown station.
  - `503` — lake unreadable (parquet missing / IO error).
- **Null handling:** apply the §2.1 sentinel rule; omit-or-`null` is acceptable but be consistent (recommend explicit `null`).

---

### 4.1 `GET /weather/observations/latest` — newest hourly observation

Replaces operation #1.

**Query params**

| Param     | Req | Default | Notes              |
|-----------|-----|---------|--------------------|
| `station` | no  | 3761    | WBANNO             |

**Responses**
- `200` — single `EnvironmentalObservation` (§5.1).
- `204` — no observations for station.

---

### 4.2 `GET /weather/observations` — hourly observations over a range

Replaces operations #2 (and the raw-read half of #3).

**Query params**

| Param      | Req | Default | Notes                                                                 |
|------------|-----|---------|-----------------------------------------------------------------------|
| `station`  | no  | 3761    | WBANNO                                                                 |
| `start`    | yes | —       | ISO-8601 UTC, inclusive                                                |
| `end`      | yes | —       | ISO-8601 UTC, inclusive                                                |
| `order`    | no  | `desc`  | `desc` (newest-first, **default — current callers rely on this**) or `asc` |
| `interval` | no  | `1`     | Server-side downsample: keep every Nth hourly row. Used by consumers as 1 (7d), 6 (30d), 24 (90d) to bound payload size. |

**Response `200`**
```json
{
  "station": 3761,
  "order": "desc",
  "interval": 1,
  "count": 168,
  "observations": [ /* EnvironmentalObservation, §5.1 */ ]
}
```

---

### 4.3 `GET /weather/daily` — daily aggregates over a range

Replaces operations #4, #5, #6 (one endpoint, field-selected).

**Query params**

| Param       | Req | Default                                  | Notes                                                                 |
|-------------|-----|------------------------------------------|-----------------------------------------------------------------------|
| `station`   | no  | 3761                                     | WBANNO (see §2.2 note on gold station filtering)                      |
| `start`     | yes | —                                        | `YYYY-MM-DD`, inclusive                                               |
| `end`       | yes | —                                        | `YYYY-MM-DD`, inclusive                                               |
| `min_hours` | no  | `12`                                     | Quality gate: include only days with `hours_observed >= min_hours`. Set `0` to disable. |
| `fields`    | no  | all                                      | CSV of: `soil_temp_10_f_mean,air_temp_avg_f,air_temp_max_f,air_temp_min_f,gdd50,hours_observed`. |

**Null-guard semantics (preserve current behavior):** a day is **included** only if **every
requested field** in `fields` is non-null (this matches how the three source operations each
filtered on the columns they projected). `air_temp_max_f`/`air_temp_min_f` are the exception
when requested *alongside* `gdd50` for the GDD use case — see the note below.

> **GDD compatibility note:** operation #6 (daily GDD) filters only on `gdd50 IS NOT NULL` and
> tolerates null `air_temp_max_f`/`air_temp_min_f` (it defaults them to 0 for display). To
> reproduce this exactly, when `fields` includes `gdd50`, treat `gdd50` as the only required
> non-null field and return `air_temp_max_f`/`air_temp_min_f` as `null` when missing. When
> `gdd50` is **not** requested, require all requested fields to be non-null (operations #4/#5
> behavior). Document whichever rule you implement; the consumer relies on: *#4/#5 drop
> null-soil/air days; #6 keeps any day with a gdd50.*

Rows always ordered by `day` **ASC**.

**Response `200`**
```json
{
  "station": 3761,
  "start": "2026-01-01",
  "end": "2026-06-27",
  "min_hours": 12,
  "count": 178,
  "days": [ /* DailyAggregate, §5.2 */ ]
}
```

---

### 4.4 `GET /weather/gdd/ytd` — growing-degree-days year-to-date (derived)

Replaces operation #7. Convenience over `/weather/daily?fields=gdd50`.

**Query params**

| Param     | Req | Default       | Notes                                  |
|-----------|-----|---------------|----------------------------------------|
| `station` | no  | 3761          | WBANNO                                 |
| `year`    | no  | current year  | Calendar year                          |
| `base`    | no  | `50`          | GDD base °F. Only `50` is precomputed; reject others or compute on the fly. |

Semantics: sum `gdd50` for `day` in `[Jan 1 of year, today]` (or `[Jan 1, Dec 31]` for past
years). If `year` is in the future relative to today, return `204`.

**Responses**
- `200`
  ```json
  { "station": 3761, "year": 2026, "base_f": 50,
    "cumulative_gdd": 812.5, "through": "2026-06-27", "days_counted": 178 }
  ```
- `204` — no data / future year.

---

### 4.5 `GET /weather/summary` — 7-day summary + trend (derived)

Replaces operation #3 / §3.1. Optional but recommended (dashboard hot path).

**Query params**

| Param     | Req | Default | Notes                                  |
|-----------|-----|---------|----------------------------------------|
| `station` | no  | 3761    | WBANNO                                 |
| `window`  | no  | `7d`    | Averaging window. `7d` is the only one consumers use today. |

**Response `200`** — `EnvironmentalSummary` (§5.3). All numeric fields nullable when the
window has no qualifying data; `current` is `null` if no latest observation.

---

### 4.6 `GET /weather/health` — connectivity / readability probe

Replaces operation #8.

**Response**
- `200` `{ "status": "ok", "layers": { "silver": true, "gold": true } }`
- `503` `{ "status": "degraded", "layers": { "silver": true, "gold": false } }`

Each layer flag = "a trivial `SELECT 1 ... LIMIT 1` against that parquet returned a row."

---

## 5. Response schemas

### 5.1 `EnvironmentalObservation` (silver, native units)

```jsonc
{
  "obs_ts_utc": "2026-06-27T14:00:00Z",   // string, ISO-8601 UTC
  "station": 3761,                         // int (WBANNO)
  "soil_temp_c": {                         // °C, each nullable
    "5":  21.3, "10": 20.8, "20": 20.1, "50": 19.4, "100": 18.9
  },
  "soil_moisture": {                       // volumetric 0–1, each nullable (negatives→null)
    "5":  0.22, "10": 0.24, "20": 0.26, "50": null, "100": null
  },
  "air_temp_c": 24.6,                      // °C, nullable
  "rh_pct": 58.0,                          // %, nullable (negatives→null)
  "precip_mm": 0.0                         // mm, nullable (negatives→null)
}
```

> Depth-keyed maps preserve all five depths. The primary consumer mostly uses the `10` cm
> series for soil temp/moisture, but the API should expose all five for completeness.

### 5.2 `DailyAggregate` (gold, °F)

```jsonc
{
  "day": "2026-06-27",          // date
  "station": 3761,              // int (or omit if gold is single-station)
  "soil_temp_10_f_mean": 69.4,  // °F, nullable
  "air_temp_avg_f": 76.2,       // °F, nullable
  "air_temp_max_f": 88.1,       // °F, nullable
  "air_temp_min_f": 64.3,       // °F, nullable
  "gdd50": 20.2,                // base-50°F GDD, nullable
  "hours_observed": 24          // int
}
```
Only requested `fields` need be present; unrequested fields may be omitted.

### 5.3 `EnvironmentalSummary` (derived, °F)

```jsonc
{
  "station": 3761,
  "window": "7d",
  "last_updated": "2026-06-27T14:05:00Z",
  "current": { /* EnvironmentalObservation, §5.1, or null */ },
  "soil_temp_10_avg_f": 68.9,   // nullable
  "ambient_temp_avg_f": 74.1,   // nullable
  "humidity_avg": 61.3,         // nullable
  "precip_total_mm": 12.4,      // nullable (0 if no precip but window had data)
  "soil_temp_trend": "rising"   // "rising" | "falling" | "stable" | "unknown"
}
```

---

## 6. Mapping table — current operation → endpoint

| Current in-process operation         | New endpoint                                   |
|--------------------------------------|------------------------------------------------|
| latest hourly reading                | `GET /weather/observations/latest`             |
| hourly readings in [start,end]       | `GET /weather/observations?start&end&order&interval` |
| 7-day summary + trend                | `GET /weather/summary?window=7d`               |
| daily soil-temp averages             | `GET /weather/daily?fields=soil_temp_10_f_mean` |
| daily paired air+soil averages       | `GET /weather/daily?fields=air_temp_avg_f,soil_temp_10_f_mean` |
| daily GDD rows                       | `GET /weather/daily?fields=air_temp_max_f,air_temp_min_f,gdd50` |
| GDD year-to-date                     | `GET /weather/gdd/ytd?year=YYYY`               |
| connectivity probe                   | `GET /weather/health`                          |
| daily disease-model inputs (op 9)    | **none yet** — see §3.3                        |

---

## 7. Implementation notes for the new service

- **Reading the lake:** the service owns the parquet reads. DuckDB (`read_parquet(...)`) is
  the path-of-least-resistance and matches the current engine exactly; Polars or a loaded
  columnar DB also work. Keep file paths in config:
  - `DATALAKE_ROOT` (default `/data`); derive
    `…/silver/weather/silver_weather.parquet` and `…/gold/weather/daily_weather.parquet`,
    each overridable by an explicit `WEATHER_SILVER_PATH` / `WEATHER_GOLD_PATH`.
  - `DEFAULT_STATION_WBANNO` (default `3761`).
- **Silver station filter:** `CAST(wbanno AS INTEGER) = :station` (wbanno may be stored as
  string).
- **Concurrency:** DuckDB connections are not thread-safe to share; open a short-lived
  connection per request (or use a small pool), and run blocking reads off the async event
  loop. The current consumer opens a fresh in-memory DuckDB connection per query.
- **Performance:** local parquet scans are fast; caching is optional. If you add caching,
  respect the consumers' freshness expectations (sensor data treated as stale after ~5 min).
- **Parameterization / injection:** bind `start`/`end`/`station` as parameters. The only
  value interpolated into SQL today is the trusted file path (single-quote-escaped); keep
  user input parameterized.
- **Determinism:** `/daily` ordering is always `day ASC`; `/observations` defaults to
  `obs_ts_utc DESC`. Preserve these — consumers depend on ordering (e.g. trend math reads
  "newest-first").

---

## 8. Out of scope

- Writing to the lake (this is read-only; Dagster owns ingestion).
- Bronze-layer access.
- Forecast data and patio-sensor data (those come from other sources — OpenWeatherMap and
  Home Assistant — and are **not** part of this lake API).
- Authentication provider details (left to the implementing team).

---

## 9. Suggested build order

1. Config + lake connectivity + `GET /weather/health`.
2. `GET /weather/observations/latest` and `…/observations` (silver, §5.1) — get the
   native-unit reads and sentinel normalization right first.
3. `GET /weather/daily` with field selection + the GDD null-guard rule (§4.3).
4. Derived `GET /weather/gdd/ytd` and `GET /weather/summary` (§3.1 trend math).
5. OpenAPI 3.1 document + contract tests asserting ordering, null-normalization, and the
   quality gates (`hours_observed >= 12`, sentinel `< 0 → null`).
