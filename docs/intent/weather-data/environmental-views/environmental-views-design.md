---
parent: weather-data
prefix: WX-VIEW
---

# Environmental Views

## Context and Design Philosophy

This leaf owns the read-only views of measured conditions: the Environmental page
(`/environmental`), the `GET /api/v1/historical` time series behind its charts, the rendering of
`GET /api/v1/environmental` (current conditions), the `Gauge` and `TrendChart` components, and
the contract of the lake's connection probe that `/health` reports. It holds no agronomic
logic. Each number shown comes from a weather-sources contract or from data sync's cached
summary.

Principles:

- **Measured over modeled.** History is the station's record (silver hourly, gold daily GDD),
  including its gaps. The patio sensor has no history, so ambient and humidity history is the
  station's.
- **One answer everywhere.** The page shows conditions. Questions that a feature component
  answers (pre-emergent, seeding, disease risk) are not restated here with other numbers.
- **Withhold rather than invent.** A missing value renders as `--` and an empty series as "No
  data available". Lag is disclosed rather than hidden.

The 10 cm soil-temperature outlook shown on this page (`PredictionChart`, its crossing cards)
is owned by the soil-temperature-outlook leaf. Data sync owns `/environmental`'s refresh logic
and the `/health` endpoint itself.

## API Endpoints

| Method | Path | Behavior | Errors |
|---|---|---|---|
| GET | `/api/v1/environmental` | Data sync's `get_or_refresh()`: the cached `EnvironmentalSummary`, refreshed when stale (`backend/src/api/environmental.rs`) | Propagates refresh errors (in practice none: source failures degrade inside the refresh) |
| POST | `/api/v1/environmental/refresh` | `force_refresh()`: refetches every source | same |
| GET | `/api/v1/historical?range=7d\|30d\|90d` | See below (`backend/src/api/historical.rs`) | 400 for a bad range; 503 with the lake-not-configured sentence when the lake client is missing; 503 with the lake-unreadable sentence (detail logged) for a lake error |

### `/historical`

- **Hourly series.** Weather sources' `fetch_range(now − range, now)` from silver, reversed to
  ascending. 7d keeps every hour. 30d and 90d group the hours into 6-hour and 24-hour time
  buckets (aligned to UTC, each point stamped at its bucket's start): precipitation is summed
  and the other series are averaged over the hours that reported them (`series()`). Series:
  10 cm soil temperature, air temperature, RH, 10 cm moisture, precipitation (values ≥ 0). A
  missing value is left out of its series, not zero-filled; a bucket with no value for a
  series is left out of that series. Default range: 7d.
- **GDD.** `fetch_daily_gdd` from Jan 1 of the range-start year to today, accumulated with a
  reset each Jan 1 (agronomic rules' `gdd::accumulate_daily_gdd`), then filtered to the range
  and stamped 12:00 UTC. "Today" and the year are the lawn's local date (parent decision):
  `gdd_window(&clock, start, now)` (`backend/src/api/historical.rs`) turns the range's instants
  into local dates with the app's `Clock`, giving Jan 1 of the local range-start year to
  accumulate from, the first date shown and the last date (the local today).

## Lake Connection Probe

`WeatherLakeClient::test_connection` (`backend/src/datasources/weather.rs`) runs
`SELECT 1 FROM <file> LIMIT 1` against the silver and then the gold parquet and returns `true`
only when both reads succeed. Any error reads as `false`; the probe never fails. Data sync's `check_connections()` reports it under the JSON key
**`soildata`** in `/health`, the dashboard connection dots and MCP `lawn_snapshot`; the UI
labels it "Weather lake". The Home Assistant and OpenWeatherMap probes belong to weather
sources' clients; when and how often the probes run belongs to data sync (whose design records
that they ignore the staleness limits).

## Environmental Page

`frontend/src/pages/Environmental.tsx`, route `/environmental`.

- **Loading.** Loads `/environmental` and `/soil-temp-forecast` together. A forecast failure
  becomes `null` and hides its section. The page polls every 30 s **only while the tab is
  visible**, pauses when hidden, and refetches when the tab becomes visible. "Refresh Now"
  calls the POST endpoint. Errors show in a banner as `describeError(e)` (Web UI's
  human-readable errors), and stale responses are discarded with an `AbortController`.
  `/historical` loads separately for the selected range; a failed load clears the trend charts
  and shows its own plain-language error in their place, so a range change never leaves the
  previous range's data under the new selector.
- **Header.** "Last updated" (`last_updated`), the soil trend, and "Station soil observed …"
  (`soil_observed_at`, the station's own time for the soil values). When data sync marks the
  soil stale (`soil_stale`), a banner says when the station's soil data stopped and that soil
  readings and soil-based advice are paused, worded as on the dashboard.
- **Gauges.** Four `Gauge`s (`frontend/src/components/Gauge.tsx`, configs in
  `components/gaugeConfigs.ts`, `gaugeLevel`). The bar is green, turns amber at `warn` and red
  at `critical` — and, on a gauge with a dry side, amber below its dry `warn` and red below its
  dry `critical`. A warning or critical state is also said in words beside the value ("High",
  "Very high", "Dry", "Very dry"), and the numeric value is always shown (`--` when null).

| Gauge | Range | Warn / critical | Dry warn / critical |
|---|---|---|---|
| Soil Temp (10 cm) °F | 30–100 | 75 / 85 | — |
| Ambient Temp °F | 0–110 | 85 / 95 | — |
| Humidity % | 0–100 | 80 / 90 | — |
| Soil Moisture (10 cm) % (fraction × 100) | 0–50 | 40 / 45 | below 15 / below 10 |

- **Summary.** Four 7-day summary cards (precipitation in inches), a soil profile table for
  5–100 cm, and **5-Day Forecast** cards for every `daily_summary` day (high/low, RH,
  precipitation in inches, max `pop`). A day the forecast covers for fewer than 24 hours
  (`hours_covered`) is marked "partial day".
- **Soil Temperature Forecast.** The outlook leaf's `PredictionChart` and crossing cards.
- **Historical Trends.** A 7d / 30d / 90d selector over six `TrendChart`s
  (`components/TrendChart.tsx`, a Recharts area chart, "No data available" when empty, an
  optional dashed red reference line). Reference lines (`TREND_REFERENCES` in
  `components/environmental/display.ts`): air at 85 °F "Heat stress", RH at 80 % "Humid (80%)"
  and moisture at 0.10 "Drought". Soil temperature, GDD and precipitation have none:
  soil-temperature and GDD milestones are the timing windows' and the outlook's questions, and
  disease risk comes from the per-disease models.

`frontend/src/utils/units.ts`: `mmToInches` and `formatInches` (25.4 mm per inch). The page is
the only place precipitation becomes inches.

## Failure and Degradation

| Failure | Effect |
|---|---|
| Lake unreadable | `/historical` returns 503. The trend section clears its charts and shows the lake-unreadable sentence. The `soildata` dot turns false when either silver or gold is unreadable. Current conditions stay at data sync's last good summary |
| Home Assistant down | Ambient and humidity gauges show `--`. History is unaffected |
| OpenWeatherMap down | The 5-Day Forecast cards disappear once data sync's reuse window expires |
| `/soil-temp-forecast` fails | Its section is hidden; the rest of the page renders |

## Decisions & Alternatives

| Decision | Chosen | Alternatives Considered | Rationale |
|---|---|---|---|
| Health key name | Lake connectivity reported as `soildata` | Rename to `lake` / `weather` | Kept for frontend compatibility (owner's lake-migration decision). The UI labels it "Weather lake". |
| Environmental page refresh | 30 s polling, visibility-aware, plus a manual force-refresh | Push or SSE; no polling | Data sync's staleness limits keep polling cheap, and a hidden tab makes no calls. [inferred] |
| `/historical` point budget | Aggregate longer ranges into 6 h (30d) and 24 h (90d) time buckets: precipitation summed, other series averaged | Return every hour; keep every Nth row | Keeps charts responsive while rain totals stay true and spacing stays even when hours are missing. |
| Reference lines | Heat stress, Humid (80%), Drought only | Lines for pre-emergent, crabgrass and "disease risk" | One answer everywhere: those questions belong to timing windows and disease risk, at their own depths and models. |
| Lag disclosure | Soil observation time and the 72 h stale banner, as on the dashboard; partial forecast days labelled | `last_updated` only | `last_updated` is the app's clock; the station's lag is the fact the owner needs (lag carried as data). |
| Gauge warnings | A text label ("High", "Dry") beside the colored bar | Color alone | State is never carried by color alone (Web UI convention). |
| Moisture gauge | Warns wet (40 / 45 %) and dry (amber < 0.15, red < 0.10) | Wet only | Drought is the more common action; the thresholds are the irrigation rule's. |
| Lake probe | One-row reads of silver and gold | Gold only; a full query | Silver backs the live reading and `/historical`; a gold-only probe can report a broken lake as connected. One row each keeps it cheap. |

## Open Questions & Future Decisions

None.

## References

- Parent: `docs/intent/weather-data/weather-data-design.md`.
- `docs/intent/weather-data/weather-sources/`: `fetch_range`, `fetch_daily_gdd`, the summary
  shape.
- `docs/intent/weather-data/soil-temp-outlook/`: the chart and crossing cards on this page.
- `docs/intent/data-sync/`: refresh, `check_connections()`, `/health`.
- `docs/intent/web-ui/`: shell, shared styles, status-palette and table-view conventions.
- Code: `backend/src/api/{environmental,historical}.rs`, `backend/src/models/historical.rs`,
  `backend/src/datasources/weather.rs` (`test_connection`), `frontend/src/pages/Environmental.tsx`,
  `frontend/src/components/{Gauge,TrendChart}.tsx`, `frontend/src/components/gaugeConfigs.ts`,
  `frontend/src/components/environmental/display.ts`,
  `frontend/src/utils/units.ts`.
