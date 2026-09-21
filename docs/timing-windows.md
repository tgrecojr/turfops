# Seeding & Pre-Emergent Timing Windows

`GET /api/v1/timing-windows` answers three start/end-of-season questions from the
station's own record rather than from calendar dates:

- When should I **seed or overseed** (fall, with spring and dormant seeding as fallbacks)?
- When should the **spring pre-emergent** go down?
- When should the **fall pre-emergent** go down — and should it at all this year?

Code: `backend/src/logic/timing/` (pure logic), `backend/src/api/timing.rs` (handler),
`backend/src/datasources/weather/climatology.rs` (lake query),
`backend/src/models/timing.rs` (response types).
UI: `frontend/src/pages/Timing.tsx` (`/timing`), `frontend/src/components/timing/`,
`frontend/src/types/timing.ts`; the dashboard shows `TimingWidget`.

## Inputs

| Input | Source | Notes |
|-------|--------|-------|
| Daily mean soil temperature at **5 cm** | Lake silver hourly `soil_temp_5`, grouped by `obs_date_local`; days with < 12 hourly readings are dropped | Germination thresholds are defined at ~2 in. The gold layer only aggregates the 10 cm probe, which lags 5 cm by roughly a week in both spring and fall. |
| Daily min / mean air temperature, `gdd50` | Lake gold `daily_weather.parquet` (`hours_observed >= 12`) | Min air temp drives freeze dates; mean air temp drives the soil forecast and the 30-day anomaly. |
| 5-day forecast | OpenWeatherMap daily summary | Converted to 5 cm soil estimates with the existing lagged air→soil regression (`logic/soil_temp_prediction`), refit on the last 30 days of 5 cm data. Station air and forecast air are joined into one daily series, with the hole between them (the lake trails by a day or more; the forecast starts today) linearly interpolated up to 7 days — otherwise every outlook day whose lagged driver falls in the hole is lost. The outlook is shifted by the model's error on the last observed day so it continues from the measured value. |
| Application log | Postgres | `Overseed` and `PreEmergent` entries mark windows done or blocked. |

Up to **15 calendar years** are read (`HISTORY_YEARS`). A year counts toward the typical
dates only if it is complete (before the current year) and has ≥ 300 days of 5 cm soil
data. The read is memoized in-process for one hour (`AppState::climate_cache`); nothing
is cached in Postgres, so the current season is always evaluated on current data.

## Method

1. **Smooth.** A trailing 5-day mean by calendar date (≥ 4 of the 5 days present).
2. **Detect sustained crossings.** A boundary such as "5 cm soil cools to 70°F" is the
   first day in its scan range from which the smoothed value stays at/past the
   threshold for **5 consecutive days**. A data gap or a day back across the threshold
   resets the run. The test is state-based, so a season that starts already past the
   threshold opens on the first day of the scan range.
3. **Typical dates.** Each boundary is located in every historical year (a crossing
   whose first day comes straight after a gap in the data is skipped for that year — it
   dates the sensor outage, not the soil), then
   summarized as median, 10th/90th percentile (nearest rank), earliest and latest.
   Confidence: High ≥ 8 years, Medium ≥ 4, else Low.
4. **This season.** Each boundary is resolved to one of:

   | Source | Meaning | Counts as passed? |
   |--------|---------|-------------------|
   | `Observed` | Seen in station data and held 5 days | yes |
   | `Tentative` | Seen in station data, run still in progress (< 5 days) | yes — the detail text says "held N of 5 days". On a **closing** boundary it makes the window `Closing`, never `Closed` (see State) |
   | `Forecast` | Only the soil forecast crosses it. The outlook starts the day after the last station day, so a crossing inside the lake's lag is dated *today*, never in the past | no → `OpeningSoon` |
   | `Typical` | Not seen: the historical median is used | only when the median is still ahead it is an estimate; if the median has passed **and the station has reported past it** with no crossing, the boundary is **overdue** and carries *no date* ("later than usual"). Inside the lake's 1–2 day lag the median stays the estimate. The detail text only says "running behind" past the 90th percentile — the median alone is passed in half of all years |

   When station soil data is more than 5 days old, or a boundary's scan range has
   ended, typical dates are trusted by the calendar instead, and a data note says so.
   With stale data only a *confirmed* crossing still counts: a run that was in progress
   when the station went quiet, or one seen only in the forecast, falls back to the
   calendar as well — nobody knows whether it held.
5. **State.** `Closed` once the closing boundary passed *and held* — a tentative close
   reads `Closing` ("threshold reached, held N of 5 days"), so one warm day cannot
   declare spring pre-emergent missed and then reopen it after the next front;
   `NotYet`/`OpeningSoon` (within
   7 days or in the forecast) before opening; inside the window `Open`, `Ideal` or
   `Closing` depending on the ideal stretch. The application log then overrides:
   `Done` if the window's own application is logged, else `Blocked` on a conflict.

Freeze dates are the last day before July / first day from August with a minimum air
temperature ≤ 32°F. A year's freeze date only counts when the record *around* it is
sound: ≥ 90 % of days present from Sep 15 up to a first fall freeze (a hole there could
hide an earlier one), or from a last spring freeze through May 31. What happens on the far
side is irrelevant — the real station lost most of December 2018, which must not discard
the Oct 22 freeze it plainly recorded. Freeze-anchored boundaries always use the *typical*
(median) first freeze, never the current year's actual or forecast freeze — by the time
a freeze is in the forecast it is far too late to seed.

## Windows

All soil thresholds are 5-day means at 5 cm. Constants live in `logic/timing/windows.rs`.

| Window | Opens | Ideal stretch | Closes |
|--------|-------|---------------|--------|
| **Fall seeding & overseeding** (cool-season) | Soil cools to **75°F**, scanned from Aug 15 | until **45 days** before the typical first fall freeze | the earlier of **30 days** before the typical first freeze and soil cooling to **55°F** |
| **Spring pre-emergent** | Soil warms to **45°F** (from Feb 1) | from **50°F** | the earlier of soil warming to **55°F** and **200 GDD** (base 50°F) |
| **Fall pre-emergent** | Soil cools to **70°F** (from Aug 1) | until **65°F** | Soil cools to **55°F** |
| **Spring seeding** (secondary) | Soil warms to **50°F** | — | Soil warms to **65°F** |
| **Dormant seeding** (optional) | Soil cools to **40°F** (Nov 1 – Jan 31) | — | Soil warms back to **45°F** the following spring |

Establishment buffers before the first freeze vary by grass: Kentucky bluegrass 60/45
days (slow to germinate), tall and fine fescue 45/30, perennial ryegrass 35/21.
Warm-season and mixed lawns get the two pre-emergent windows only.

Spring windows roll to next year from August; dormant seeding belongs to the year it
opened in until July.

### Seed and pre-emergent never share a season

A pre-emergent stops grass seed as well as weed seed, so the log arbitrates:

| Logged | Effect |
|--------|--------|
| `Overseed` Jul 1 – Dec 31 | Fall seeding **Done** (through Nov 14; later is dormant seeding); fall pre-emergent **Blocked** |
| `Overseed` Nov 15 – Feb 28 | Dormant seeding **Done**; the following spring pre-emergent **Blocked** |
| `Overseed` Mar 1 – Jun 30 | Spring seeding **Done**; spring pre-emergent **Blocked** |
| `PreEmergent` Jan 1 – Jun 30 | Spring pre-emergent **Done**; spring seeding **Blocked** |
| `PreEmergent` Jul 1 – Dec 31 | Fall pre-emergent **Done**; fall and dormant seeding **Blocked** |

A spring pre-emergent does not block fall seeding, and fall overseeding does not block
the next spring's pre-emergent. No rates are ever given — the product label governs,
including its reseeding interval.

## Context

- **Anomalies:** 5 cm soil over the last 7 days and air temperature over the last 30,
  each minus the same calendar window in the history years (≥ 70 % coverage, ≥ 3
  baseline years). Descriptive only — they never shift a date.
- **Forecast alerts:** any forecast low ≤ 32°F; count of forecast highs ≥ 90°F.
- **Chart series:** every day of the current year with this year (observed, then
  forecast), last year, and the mean of the history years, all smoothed.

## Recommendations and the seasonal plan

The windows are the single source for pre-emergent and seeding timing everywhere:

- **Feed** (`logic/timing/recommendations.rs`, appended in the dashboard and
  recommendations handlers). Primary windows only. Opening soon → Info; Open / Ideal →
  Advisory; Closing → Warning; a spring pre-emergent that closed within the last 21 days
  → Warning (post-emergent advice). Fall pre-emergent is capped at Info / Advisory — it
  is the alternative to seeding, not something to push. Done, Blocked, Not yet and
  Closed produce nothing. Ids: `timing_<window>_<season_year>`. The fall seeding
  recommendation sizes the seed needed from the lawn size at 4 lbs/1000 sqft. These
  replaced the 10 cm `pre_emergent` and `fall_overseeding` rules. Core aeration and the fall-herbicide caveat also hang off the fall seeding
  window (`logic/timing/companions.rs`), and the never-firing 10 cm `SoilTempForecastRule`
  is gone (10 rules remain).
- **Seasonal plan / calendar** (`logic/timing/plan.rs`). Spring Pre-Emergent, Fall
  Pre-Emergent (new), Fall Seeding & Overseeding and Core Aeration (same window as
  seeding) use the windows' typical opens → closes medians, with earliest/latest as the
  historical range. For the season being lived now, the **status** comes from the
  window's resolved state (NotYet/OpeningSoon → Upcoming, Open/Ideal/Closing → Active,
  Closed → Missed), so a cold spring keeps pre-emergent Active past its typical close
  exactly as the Timing page does; other years use the typical dates. The handler drops
  the plan's own 10 cm version of every activity the windows answer for on that lawn
  (`plan::owned_ids`) and adds the timing ones, so when the lake is unavailable — or for
  a warm-season lawn, which has no seeding window — the legacy 10 cm activity remains. A
  window ruled out by the log is left off the plan, and because removal goes by
  `owned_ids` rather than by what came back, its legacy twin cannot reappear; a fall
  `PreEmergent` no longer marks the *spring* application complete.

A lake outage degrades the feed to no timing recommendations rather than an error.

## Page

`/timing` shows, top to bottom: data notes; tiles for the current 5 cm soil mean and the
typical last-spring / first-fall freeze; the season context and forecast alerts; a card
per **primary** window sorted most-actionable first (Closing → Ideal → Open → Opening
soon → …); the soil chart; the secondary/optional seeding windows; and a collapsible
"How this is calculated".

- **Card:** state badge (status color + symbol + label, never color alone), headline,
  the deciding-trigger sentence, a conflict box when blocked, a timeline (earliest–latest
  on record → typical window → ideal stretch, one blue ramp, with a Today marker), a
  trigger table (Trigger · Typical median with range and years · This season with its
  source) and guidance.
- A boundary with no date (overdue) reads "Later than usual — not yet seen".
- **Chart:** this year solid blue, forecast the same blue dashed, last year orange, the
  typical year a neutral dotted reference; the blue/orange pair was checked for
  color-vision-deficiency separation. Thresholds shown are the fall set from July and
  the spring set before. Defaults to −75/+60 days around today with a full-year toggle,
  and has a table view.

## How this differs from lawn-answers.com

The feature was modeled on lawn-answers.com's SeedTiming, SpringPreEm and FallPreEm
tools (reviewed 2026-09-20). Deliberate differences:

| lawn-answers | TurfOps | Why |
|--------------|---------|-----|
| ERA5-Land *modeled* soil, 0–7 cm | *Measured* USCRN soil at 5 cm | Same depth, real sensor. |
| Averages all years into one "typical year" curve, then finds one crossing | Finds the crossing in each year, then reports median and spread | Averaging hides year-to-year variability; the spread is the useful part. |
| "Live" fall seeding window closes on a forecast freeze or the 90 %-probability freeze date (it showed Nov 24 for Media, PA) | Closes 30 days before the typical first freeze or at 55°F soil, whichever is first | Seedlings need weeks of growth before a freeze; their own historical window subtracts 30 days but the live one does not. |
| Spring pre-emergent 45→50°F, no GDD | 45°F early, 50°F ideal, closed at 55°F sustained or 200 GDD | Crabgrass germinates at ~55°F in the top 2 in; 50°F closes the window too early. |
| Fall pre-emergent "optimal ~65°F" | Ideal from 70°F down to 65°F, late after that | The barrier must precede germination, which starts as soil cools through 70°F. |
| Seeding and fall pre-emergent both shown as "open" | Mutually exclusive via the application log | A pre-emergent kills the seeding. |
| NCEI 1991–2020 freeze normals (Philadelphia) | Freeze dates derived from the station's daily minimums | No new dependency; the rural station is closer to yard conditions than the airport. Fewer years, so percentiles are coarser. |
| 16-day GFS soil forecast, CPC outlook, NWS alerts | 5-day regression soil forecast; forecast freeze/heat only | No new external APIs. Open-Meteo would extend the horizon if ever adopted. |

## Caveats

- The station (USCRN PA Avondale by default) is not the yard. Soil there is measured
  under natural sod; a thin, sunny, south-facing lawn warms earlier in spring.
- With under 5 complete history years the response carries a note that typical dates
  are rough. Percentiles equal earliest/latest until there are ~10 years.
- The soil forecast reaches only as far as OpenWeatherMap's 5-day forecast and is
  omitted when the 30-day air→soil fit is weak (R² < 0.3).
- `today` is the **station-local** date, from the lake's UTC offset (read with the climate
  record). The container clock is usually UTC, which runs a day ahead every evening.
