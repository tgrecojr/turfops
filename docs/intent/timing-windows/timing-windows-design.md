---
parent: high-level-design
prefix: TIME
---

# Seeding & Pre-Emergent Timing Windows

## Context and Design Philosophy

The owner seeds, overseeds and lays down pre-emergent at the start and end of each season. This component answers "when" for each of those jobs from the station's own record instead of calendar rules of thumb. Every window has boundaries (opens, an optional ideal stretch, closes). Each boundary has a typical date, found separately in every historical year and summarized as median and spread. Each boundary also has a standing for the season being lived now: observed, tentative, forecast, or still on its typical date.

The component is the single source of seeding and pre-emergent timing. The Timing page, the dashboard widget, the recommendation feed, the seasonal plan and calendar, and the MCP tools all read the same evaluated windows. That is the HLD tenet *One answer everywhere*. Three other tenets shape the design:

- **Withhold rather than invent.** A trigger that is past its typical date but has not happened gets no date ("later than usual").
- **Measured over modeled.** Windows use measured 5 cm soil and station freeze dates. The lake's lag is disclosed rather than hidden.
- **Infer from the log.** Seeding and pre-emergent rule each other out per season, decided by the application log, never by a setting.

The full user-facing methodology lives in `docs/timing-windows.md`, which is served as an MCP resource. This LLD is the design: the structure, contracts, state machine and decisions. It cites that method doc for detail instead of repeating every threshold.

All window logic is pure: `backend/src/logic/timing/` does no I/O. The handler `backend/src/api/timing.rs` gathers the inputs.

## Spec areas

The component is one leaf. Its EARS are grouped into five spec areas under the `TIME` prefix, so a prefix grep gathers each area:

| Area | Covers |
|---|---|
| `TIME-ENG` | The window engine: boundary and window definitions, smoothing and the soil forecast, sustained-crossing detection, climatology (freezes, GDD, `date_stat`), per-year history, resolving this season, window states and text, the climate memo and "today", and the `/timing-windows` response (context, series, data notes) |
| `TIME-LOG` | Seed vs. pre-emergent exclusivity: done and conflict spans per window, Done and Blocked |
| `TIME-FEED` | Timing recommendations (severity ladder, ids, product needs), the aeration companion, fall broadleaf reconciliation, and degradation to no timing advice |
| `TIME-PLAN` | Seasonal-plan activities built from the windows: dates, status from window state, omissions, `owned_ids` |
| `TIME-UI` | The Timing page, `WindowCard`, `StateBadge`, `WindowTimeline`, `SoilSeasonChart` / table, and the dashboard `TimingWidget` |

## Inputs and the Boundary with Weather Data

| Input | Supplied by | Contract this component relies on |
|---|---|---|
| `ClimateRecord { days: Vec<ClimateDay> }` | Weather data access (`datasources/weather/climatology.rs::fetch_climate_record`) | Ascending local days from `climate_start(today)` = Jan 1 of (the lawn's current year − `HISTORY_YEARS`) (`api/timing.rs`). Each day has optional `soil_temp_5_f` (silver hourly mean, ≥ 12 h, sentinels dropped, °F), `air_min_f` / `air_avg_f` / `gdd50` (gold, `hours_observed ≥ 12`). Any field may be missing. |
| Daily forecast | Data sync (`get_or_refresh().forecast`) | Only `WeatherForecast::full_days()` is used, because a partial day's (high + low) / 2 is not a daily mean. `api/timing.rs::lake_and_forecast` (lines 61-73) supplies the lake client and these full days to both the Timing page (`compute`) and the seasonal plan (`plan_for`), so both evaluate the windows from the same inputs. |
| Application log | Applications (`queries::get_applications_for_profile_in_range`) | The feed handler reads 550 days back (`HISTORY_LOOKBACK_DAYS`), which reaches the previous July. The plan handler reads Nov 1 of year − 1 through Mar 1 of year + 1. |
| Lawn profile | Lawn profile CRUD | `grass_type` selects windows and establishment buffers. `lawn_size_sqft` sizes the seed note. |
| Air→soil regression | `logic/soil_temp_prediction::{fit_model, predict_soil_temps}` | Lagged linear fit (lag 0–7 d). Returns `None` with fewer than 14 pairs or R² < 0.3. |

**Climate memo.** `api::timing::climate_read` memoizes the record in `AppState::climate_cache` for one hour (`memo::FRESH_FOR`, `api/timing/memo.rs`). When a re-read fails, the last good record stands in while under a day old (SYNC-MEMO-002); the Timing response then opens its `data_notes` with "The weather lake could not be read just now; station history is from the last good read (<local date and time>)." (TIME-ENG-046). `climate_record` returns the record alone for the plan's timing activities. The MCP `weather_history` tool reads the same record through the same function. Nothing derived is stored in Postgres, so the current season is always evaluated on current data. The weather-data component owns the DuckDB query. This component owns the memo and every use of the record.

**Today.** `today`, the season year and every date gate in this component are the lawn's local date, `state.clock.today()` (the app's `Clock`, resolved from `TZ`, which deployment sets to the lawn's timezone), the same clock every other component uses. `compute` and `plan_for` both take it, and the climate record's history start (`climate_start(today)`) is taken from that same date. The climate record carries no date of its own.

## Boundaries and Windows

A `WindowSpec` (`logic/timing/windows.rs`) is `opens`, optional `ideal_from`, optional `ideal_until`, and `closes`. Each of those is a `Boundary`:

| Boundary | Meaning |
|---|---|
| `Soil(Crossing)` | The smoothed 5 cm soil reaches `threshold_f` in a `Direction` (Rising ≥, Falling ≤) and holds, searched from `scan_from` to `scan_to` (`SeasonDay`: month/day with a year offset of 0 or 1). |
| `BeforeFirstFreeze(days)` | That many days before the first fall freeze. |
| `Gdd(total)` | Running GDD (base 50 °F) for the season year reaches `total`. |
| `Earliest(Vec<Boundary>)` | Whichever option comes first. |

The thresholds are constants in `windows.rs` and match `docs/timing-windows.md` § Windows:

| Window (`WindowId`) | Priority | Opens | Ideal | Closes |
|---|---|---|---|---|
| `spring_pre_emergent` | Primary | rising 45 °F | from rising 50 °F | `Earliest`[rising 55 °F, 200 GDD] |
| `fall_pre_emergent` | Primary | falling 70 °F from Aug 1 | until falling 65 °F | falling 55 °F |
| `fall_seeding` | Primary | falling 75 °F from Aug 15 | until `BeforeFirstFreeze(comfortable)` | `Earliest`[`BeforeFirstFreeze(minimum)`, falling 55 °F from Aug 15] |
| `spring_seeding` | Secondary | rising 50 °F | — | rising 65 °F |
| `dormant_seeding` | Optional | falling 40 °F, Nov 1 – Jan 31 (+1) | — | rising 45 °F, Feb 1 – Jul 31 (+1) |

- Rising crossings scan Feb 1 – Jul 31. Falling crossings scan to Dec 31.
- The establishment buffers `(comfortable, minimum)` depend on the grass (`establishment_days`): KBG 60/45, PRG 35/21, everything else 45/30.
- Every lawn gets all five windows; `Mixed` is a cool-season blend. Warm-season grasses are unsupported (HLD Non-Goals).
- `season_year(id, today)`: the spring windows roll to next year from August, dormant seeding belongs to the previous year through June, and every other window uses the calendar year.

## Detection, Smoothing and Climatology

**Series** (`logic/timing/mod.rs::SeasonData::build`): the observed 5 cm points are followed by the forecast points. Then `series::smooth` takes a trailing 5-day calendar mean, which needs at least 4 of the 5 days, so gaps in the input stay gaps in the output.

**Soil forecast** (`mod.rs::forecast_points`, over the soil-temperature outlook's shared functions):
- Fit the regression (`fit_model`, pairs aligned by date) on the 30 days ending at the last soil observation.
- Build one continuous air series (`continuous_air`): station `air_avg_f` from those 30 days wins over forecast (high + low) / 2, and holes of up to 7 days are linearly interpolated.
- Predict every day after the last observed day and shift each prediction by the model's error on that day (`anchored_outlook`, the anchor). It is given the last observed day as its `today`; that date only sets each prediction's confidence, which this outlook does not use.
- The result is empty when there is no soil, no forecast, or no fit.

**Sustained crossing** (`series::detect`):
- The test is state-based: the first run in the scan range where every day is at or past the threshold.
- A day back across the threshold, or a missing date, resets the run.
- `days_held` counts only observed days. A run returns as soon as it reaches `SUSTAIN_DAYS` = 5 observed days (confirmed).
- An unconfirmed run counts only if it is still in progress on the last day of the series.
- If the series is already past the threshold on `scan_from`, the window opens on that date.
- `after_gap` marks a run whose first day follows a hole in the data, unless that first day is `scan_from`.

**Climatology** (`logic/timing/climatology.rs`):
- *History years:* complete calendar years before the current year with ≥ 300 days of 5 cm soil.
- *Freezes:* a freeze is `air_min_f ≤ 32`. The last spring freeze is the last one in Jan–Jun. The first fall freeze is the first one on or after Aug 1.
- *Freeze soundness:* a past year's freeze counts only when ≥ 90 % of days have an air minimum on the side that could hide a different date. For a fall freeze that is Sep 15 → the freeze. For a spring freeze it is the freeze → May 31.
- *GDD:* `gdd_reached` sums `gdd50` through the year. The day it reaches the target counts only if ≥ 80 % of elapsed days had a value.
- *`date_stat(events, target_year)`:* each `(season_year, date)` becomes an offset from Jan 1 of its season year, so dormant events that spill into January stay in order. Offsets are sorted. The median is the middle value (the mean of the two middle values, rounded down, for an even count). p10 and p90 are nearest-rank. It also reports earliest, latest, `sample_count`, and confidence: High with ≥ 8 years, Medium with ≥ 4, else Low. Results are returned as dates in `target_year`.

**Per-year history** (`evaluate.rs::historical_date`): this is how each boundary is dated in each history year.

| Boundary | Historical date |
|---|---|
| `Soil` | The confirmed `detect` result, dropped when `after_gap` (it dates a sensor outage) |
| `BeforeFirstFreeze(d)` | That year's own sound first fall freeze − d |
| `Gdd` | `gdd_reached` for that year |
| `Earliest` | The minimum over the options. The year is dropped if any option is missing. |

`typical()` = `date_stat` over the history years that produce a date.

## Resolving This Season

`evaluate::resolve` assigns each boundary a `Resolved { date, source, passed, days_held }`. `Season.data_fresh` is true when the last observed soil day is ≤ 5 days before today (`STALE_AFTER_DAYS`).

| Boundary / case | Result |
|---|---|
| Soil, run detected, and (data fresh or run confirmed) | Confirmed → `Observed`, passed. Unconfirmed with observed days → `Tentative`, passed, `days_held`. Forecast-only → `Forecast`, not passed, dated `max(run start, today)`, so it is never in the past. |
| Soil, no usable run (none, or an unconfirmed run with stale data) | From typical (below), with the crossing's `scan_to` |
| `BeforeFirstFreeze` | Always `Typical`: the median of the per-year freeze-anchored dates, passed once that date ≤ today. The current season never uses this year's or a forecast freeze. |
| `Gdd` | Reached this season → `Observed`, passed. Otherwise from typical, with scan end Jul 31. |
| `Earliest` | Resolve each option (`earliest_of`). Take the earliest-dated passed option that is not `Tentative`, else the earliest-dated `Tentative` option, else the earliest-dated unpassed option, else `{date: None, Typical, not passed}`. A confirmed close (an Observed 200 GDD, say) therefore wins over an earlier tentative soil touch, and the window reads Closed rather than Closing. |

**From typical** (`from_typical`):
- If the median is still ahead, or data is fresh and the station has not yet reported past the median (`last_observed < median`, the lake-lag case), the median is the estimate and the boundary is not passed.
- If the scan range has ended or data is stale, the median is used as passed (the calendar fallback).
- Otherwise the boundary is **overdue**: `date: None`, not passed. No date is invented.
- With no history the result is `date: None` as well.

## Window States

`WindowViews::weather_state` (`evaluate.rs:220-247`) derives state from the four resolved boundaries. `evaluate_window` (`mod.rs:163-195`) then applies the log on top:

1. **Closed**: closes passed and not Tentative.
2. **NotYet / OpeningSoon**: opens not passed. OpeningSoon when the opens source is `Forecast` or its date is ≤ 7 days away.
3. **Closing**: closes passed but Tentative (the threshold was reached but has not held 5 days).
4. **Open**: `ideal_from` exists and has not passed.
5. **Closing**: `ideal_until` exists and has passed.
6. **Ideal**: the window has an ideal stretch and is inside it.
7. **Open**: the window has no ideal stretch.
8. **Done / Blocked**, from the log, override all of the above: Done when the window's own application is logged, else Blocked on a conflict.

A Tentative close never yields Closed. A one-day spike past 55 °F must not declare spring pre-emergent missed and then reopen the window after the next front.

**Text** (`logic/timing/text.rs`):
- `headline` hedges non-observed dates ("around …"). An undated boundary reads "later than usual (typically …)", or "when conditions allow" with no history.
- `detail` describes the deciding boundary for the state. It says "running behind" only once today is past the boundary's p90.
- `guidance` is static per (window, state), is empty for Done and Blocked, and never states a product label rate or any pesticide rate.

## Seed vs. Pre-Emergent Exclusivity (Log Arbitration)

`log::check(id, season_year, history)` returns `Logged { done_on, conflict }`. `done_on` is the latest application of the window's own type inside its *done span*. `conflict` is set when the opposite type appears inside the *conflict span*; it is the message shown in the UI.

| Window | Own type, done span | Opposite type, conflict span |
|---|---|---|
| fall_seeding | Overseed, Jul 1 – Nov 14 | PreEmergent, Jul 1 – Dec 31 |
| dormant_seeding | Overseed, Nov 15 – Feb 28 (+1) | PreEmergent, Jul 1 – Feb 28 (+1) |
| spring_seeding | Overseed, Mar 1 – Jun 30 | PreEmergent, Jan 1 – Jun 30 |
| spring_pre_emergent | PreEmergent, Jan 1 – Jun 30 | Overseed, Nov 15 (−1) – Jun 30 |
| fall_pre_emergent | PreEmergent, Jul 1 – Dec 31 | Overseed, Jul 1 – Dec 31 |

As a result, a spring pre-emergent never blocks fall seeding, a fall overseeding never blocks the next spring's pre-emergent, and dormant seeding blocks the following spring's pre-emergent. Done is checked before Blocked (`mod.rs:171-175`).

## API Contract

| Method | Path | Response | Errors |
|---|---|---|---|
| GET | `/api/v1/timing-windows` | `TimingResponse` (`models/timing.rs`, mirrored in `frontend/src/types/timing.ts`) | 404 with no lawn profile. 503 when the lake is not configured (`not_configured(Source::WeatherLake)`: the owner sentence naming the lake, what it affects and `DATALAKE_ROOT`). Lake read and sync errors propagate (`api/timing.rs:28-35, 54-60`). |

`TimingResponse` fields:
- `generated_at`, `today` (the server-local date; see **Today**), `station`
- `soil: SoilNow {avg_5day_f, as_of, depth_cm}`: the latest observed smoothed value
- `freeze: FreezeDates {last_spring, first_fall: DateStat?, last_spring_this_year (only from June), first_fall_this_year}`
- `windows: [TimingWindow]`. Each has `id`, `name`, `priority`, `season_year`, `state`, `headline`, `detail`, `opens`, `ideal_from?`, `ideal_until?`, `closes` (each a `BoundaryView {label, typical: DateStat?, date?, source, passed, days_held?}`), `done_on?`, `conflict?` and `guidance[]`.
- `context: SeasonContext`
- `series: [SoilSeriesDay]`
- `history_years`
- `data_notes[]`

**Context** (`logic/timing/context.rs`):
- 7-day soil and 30-day air anomalies against the same calendar window in the history years. Each window needs ≥ 70 % coverage, and the baseline needs ≥ 3 years.
- One summary sentence, which prefers soil over air.
- Forecast alerts: the first forecast low ≤ 32 °F, and the count of forecast highs ≥ 90 °F.
- Anomalies are descriptive and never shift a date.

**Series:** every day of the current calendar year with this year (observed), forecast, last year, and typical (the mean of the history years, needing ≥ 3 years), all smoothed and rounded to 0.1 °F.

**Data notes** (`api/timing.rs::data_notes`) cover:
- a climate record served from the last good read, with its local read time (first, TIME-ENG-046)
- stale or absent soil data (the windows follow typical dates)
- fewer than 5 history years (typical dates are rough)
- no forecast

## Consumers: Feed, Plan, Companions

**Feed** (`api::timing::apply_to_feed`, called from `recommendation_feed::all` after the rules and disease risk). It runs `compute`, then:

1. `recommendations::to_recommendations`: Primary windows only.
   - OpeningSoon → Info.
   - Open or Ideal → Advisory. For fall pre-emergent → Info.
   - Closing → Warning. For fall pre-emergent → Advisory.
   - Spring pre-emergent Closed with `closes.date` ≤ 21 days ago → Warning.
   - Everything else produces nothing.
   - The id is `timing_<slug>_<season_year>`. The category is PreEmergent or Overseeding.
   - The title is "name — headline", the description is `detail`, the action is `guidance` (plus, for fall seeding, a seed-amount note: the overseeding amount per 1,000 sq ft from `rules/thresholds.rs::OVERSEED_RATE_LBS_PER_KSQFT` and, when the profile records `lawn_size_sqft`, the total seed for it — with no size the note gives the per-1,000 amount only, as feeding advice does (RULE-ENG-009); `recommendations.rs::seeding_rate_note`), and the data points are the 5 cm mean plus Opens, Ideal until and Closes.
   - `ProductNeed`: Herbicide · PreEmergent for the pre-emergent windows. Seed with the lawn's species targets for the seeding windows (Mixed accepts KBG / TTTF / PRG / fine fescue). The missed spring pre-emergent Warning (state Closed) declares no need: its guidance is post-emergent control, so pre-emergent never reaches the shopping list after the window has closed.
2. `companions::aeration`: the `aeration` recommendation while fall seeding is Open, Ideal or Closing.
   - Silent when the lawn was aerated since Jul 1 of the season year, and silent when seeding is Done or Blocked.
   - Warning for clay or clay-loam soil not aerated in 12 months. Advisory when not aerated in 12 months otherwise. Info when aerated within 12 months.
3. `companions::reconcile_fall_herbicide`: edits the rules engine's `broadleaf_fall`.
   - Removed when seeding is Done.
   - Capped at Advisory with an "Only if you are NOT seeding" prefix while seeding is NotYet, OpeningSoon, Open, Ideal or Closing.
   - Unchanged when seeding is Blocked or Closed.

If `compute` fails, the error is logged and the feed is left as the other sources produced it.

**Seasonal plan** (`logic/timing/plan.rs`, called from `api/seasonal_plan.rs` through `api::timing::plan_activities`):
- `activities(season, grass, applications, year)` builds `pre_emergent`, `fall_pre_emergent`, `fall_overseeding` and `core_aeration` from the typical opens and closes medians, with opens.earliest and closes.latest as the historical range and the opens confidence. `fall_overseeding` carries the same seed amount per 1,000 sq ft as the recommendation (`plan.rs:61`).
- **Status:**
  - For the season being lived now it comes from `weather_state`: NotYet or OpeningSoon → Upcoming, Open, Ideal or Closing → Active, Closed → Missed.
  - A logged own application → Completed.
  - Other years use the typical dates.
- **Omitted activities:** a window blocked by the log is left out, and aeration goes with seeding. An activity is also left out when its opens or closes has no typical date.
- `owned_ids(grass)` lists every plan id the windows answer for, including blocked ones. The plan handler removes those ids from its own 10 cm activities before adding the timing ones, so a blocked window's legacy twin cannot reappear.
- When the lake is unavailable or there are no activities, `plan_activities` returns `None` and the plan keeps its own versions. The seasonal-plan component owns that merge.
- The plan path evaluates the windows with the same inputs as the Timing page, the forecast's full days included (`SeasonData::build(days, forecast, …)`, `api/timing.rs:196-197`), so the current season's status matches the window's state on the page.

**MCP:** the `timing_windows` tool and `lawn_snapshot` call `get_timing_windows` and drop `series` (`mcp/trim.rs::timing_without_series`).

## UI

The UI files are `frontend/src/pages/Timing.tsx` (route `/timing`, nav "Seed & Pre-Em Timing"), `frontend/src/components/timing/*` and `frontend/src/types/timing.ts`.

**Page order:**
1. Data notes.
2. Tiles: the 5 cm 5-day mean with as-of date; the last spring freeze median with "as late as p90"; the first fall freeze median with this year's freeze if seen, else "as early as p10".
3. Context summary and ⚠ forecast alerts.
4. Primary `WindowCard`s sorted by `byUrgency` (Closing, Ideal, Open, OpeningSoon, NotYet, Done, Blocked, Closed).
5. `SoilSeasonChart`.
6. "Other seeding options" (Secondary and Optional cards).
7. `TimingMethod`, a collapsible "How this is calculated".

A failed fetch shows `describeError(e)` (Web UI's human-readable errors): with no lawn profile, a pointer to set one up in Settings; with the weather lake unavailable or not configured, the server's sentence naming the lake; never a status code, JSON or internal detail.

**Conventions:**
- **`StateBadge`:** the window-state color is never used alone. Every badge is symbol + label (○ ◔ ● ★ △ ✕ ✓ ⊘) on a tint of the status color, with ink text. Colors are the status palette plus neutrals in `WINDOW_STATE_COLORS`.
- **`WindowCard`:**
  - Shows the name · season year, headline, badge, conflict box, detail, timeline, a trigger table (Trigger · Typical median with earliest–latest and years · This season with source and "held N of 5 days"), the typical-date confidence, and guidance.
  - A boundary with `date: null` and a typical date reads "Later than usual — not yet seen", never a date.
- **`WindowTimeline`:**
  - One blue ramp: earliest–latest on record, then typical opens → closes, then the ideal stretch.
  - A Today marker, and an `aria-label` that gives the typical dates.
  - Hidden when the opens or closes typical date is missing.
- **`SoilSeasonChart`:**
  - This year `#2a78d6` solid, the forecast the same hue dashed and bridged onto the last observed day, last year `#eb6834`, and the typical year neutral dotted.
  - The blue/orange pair was checked for color-vision-deficiency separation.
  - 10 °F ticks.
  - Threshold lines show the fall set from July and the spring set before.
  - Defaults to −75 / +60 days around today, with a full-year toggle.
  - `SoilSeasonTable` provides the table view.
- **`TimingWidget`** (dashboard): the Primary windows by `byUrgency` with headline and badge, linked to `/timing`. It is hidden when its fetch fails.

## Failure and Degradation

| Condition | Behavior |
|---|---|
| Lake not configured or unreadable | `/timing-windows` returns an error (503 when not configured). The feed adds no timing recommendations, no aeration companion and no herbicide reconciliation. The plan keeps its own activities. |
| Station soil > 5 days old | Only confirmed crossings count. Other boundaries follow the calendar (a passed median counts as passed). A data note says so. |
| Inside the lake's 1–2 day lag | A typical date the station has not yet reported past stays an estimate, not overdue. A forecast crossing in the lag is dated today. |
| No forecast, or a weak regression fit | No forecast points: nothing is OpeningSoon from the forecast. A data note covers the missing-forecast case. |
| Fewer than 5 history years | Data note. Percentiles collapse toward earliest and latest. |
| No history | No typical dates. Boundaries are undated ("when conditions allow"). Plan activities are absent. |
| Sensor outage inside a past season | That year's `after_gap` crossing is dropped from the typical dates. A freeze whose near side is < 90 % complete is dropped. |

## Decisions & Alternatives

| Decision | Chosen | Alternatives Considered | Rationale |
|---|---|---|---|
| Soil depth | 5 cm daily mean from silver hourly | Gold 10 cm (precomputed) | Germination thresholds are defined at about 2 in. 10 cm lags 5 cm by about a week in both seasons. Owner decision. |
| Typical dates | Locate each boundary per year, then median / p10 / p90 / earliest / latest | One averaged typical-year curve, crossed once | Averaging hides year-to-year spread and can cross on a date no year did. The spread is the useful part. |
| Freeze source | Station gold `air_temp_min_f ≤ 32`, soundness-checked | NCEI 1991–2020 normals (Philadelphia airport) | No new dependency. The rural station is closer to yard conditions. Owner decision. |
| Freeze anchor | The current season uses the typical (median) freeze. Past years are dated from their own freeze. `Earliest` drops a year missing any trigger. | A forecast freeze; the typical freeze for every year | A forecast freeze is too late to act on. Dating past years from the typical freeze collapsed the spread to one date. The earlier of two dates is unknown while one is missing. |
| Seeding close | min(typical first freeze − establishment buffer, soil 55 °F) | Close on a forecast or 90 %-probability freeze | Seedlings need weeks of growth before freezing weather. |
| Spring pre-em close | 55 °F sustained or 200 GDD, whichever first; 45 early / 50 ideal | Close at 50 °F | Crabgrass germinates at about 55 °F in the top 2 in. Closing at 50 °F closes the window too early. |
| Fall pre-em window | Ideal 70 → 65 °F, closed at 55 °F | "Optimal about 65 °F" | The barrier must precede germination, which starts as soil cools through 70 °F. |
| Confirmation | 5-day trailing mean, held 5 observed days; forecast days never confirm | Single-day threshold; counting forecast days | Filters February thaws and September cold snaps. A forecast is not evidence. |
| Tentative close | Reads Closing (Warning), Closed only once confirmed | Closed on first touch | One warm day must not declare the window missed and then reopen it. Owner decision. |
| Overdue boundary | No date; "running behind" only past p90 | Show the median; flag behind at the median | Withhold rather than invent. The median is passed in half of all years. |
| Lake lag | Not overdue until the station has reported past the median; a forecast crossing in the lag is dated today | Judge against today | A crossing cannot be late on days the station has not reported. |
| Stale data | Over 5 days stale: only confirmed crossings count, otherwise the calendar | Trust in-progress or forecast runs | Nobody knows whether a run in progress when the data stopped held. |
| Outage crossings | Drop `after_gap` crossings from history | Keep them | A run starting right after a hole dates the outage, not the soil. |
| Seed vs. pre-emergent | Mutually exclusive per season, inferred from the log | A per-season setting; show both open | A pre-emergent kills the seeding. The log already records which one happened. Owner decision. |
| Cache | Climate record memoized 1 h in memory | Postgres crossings cache | The crossings cache never refreshes the current year. The lake gains only a day at a time. Owner decision. |
| Placement | A dedicated Timing page that feeds the seasonal plan | Growing `seasonal_plan.rs` | Owner decision. |
| Forecast horizon | 5-day OWM via the air→soil regression | 16-day Open-Meteo / GFS soil | No new external API. Deferred by the owner. |
| Single source for timing advice | The feed, plan, aeration and fall herbicide all derive from window state; the plan removes `owned_ids` | Separate 10 cm rules beside the windows | One answer everywhere. Owner decision. |
| Aeration and fall broadleaf | Follow the fall seeding window and the log | Calendar or 10 cm rules | Cores and seed go down together. Herbicide is right only for a lawn not being seeded. Owner decision. |
| Plan status | The current season comes from the resolved window state | Typical dates only | A cold spring must keep pre-emergent Active past its typical close, as on the Timing page. |
| Fall pre-em severity cap | Info / Advisory | Same ladder as the other windows | It is the alternative to seeding, not something to push. |
| Missed spring pre-em | Warning for 21 days after close, with no product need | Silent once closed; keeping the pre-emergent need | Points the owner to post-emergent control; pre-emergent is no use once the window has closed, so it must not reach the shopping list. |
| Plan evaluation | The same inputs as the Timing page, forecast included | The plan without the forecast | One answer everywhere: the plan's status must match the page's state. |
| Composite close with a tentative option | A passed non-Tentative option wins over an earlier Tentative one | The earliest passed option regardless | An observed 200 GDD close is firm; a tentative soil touch must not turn Closed back into Closing. |
| Sync failure during timing | Evaluate without the forecast, with a data note | Fail the request | The lake read does not depend on the forecast; losing one source degrades only what depends on it (HLD). |
| Feed scope | Primary windows only | All windows | Secondary and optional seeding are fallbacks shown on the page. [inferred] |
| Recommendation id | `timing_<slug>_<season_year>` | Id without the year (relying on answered-state expiry) | Each season's window is a distinct episode. Answering this spring's window must not touch next spring's. [inferred] |
| Spring scan start | Feb 1 | Jan 1 | Keeps a January thaw from opening a spring window. [inferred] |
| Seeding scan start | Aug 15 | Aug 1 like fall pre-emergent | Seeding before mid-August puts seedlings into peak heat and disease pressure. [inferred] |
| Season rollover | Spring windows roll in August; dormant seeding belongs to its opening year through June | Calendar year | Keeps the next actionable season in view after the current one ends. [inferred] |
| Percentiles and confidence | Nearest-rank; High ≥ 8 years, Medium ≥ 4 | Interpolated percentiles | Simple and reproducible in SQL for independent checks. [inferred] |
| Today | The lawn's local date (the app's `Clock`, `TZ` set to the lawn's timezone); the history start year from the same date | The UTC date; a station-local date from the lake's UTC offset | Every timing date (a crossing day, a freeze night, a season year) is a lawn-local calendar day. One configured clock gives this component the same day as every other one. |
| Seed amount in advice | The fall seeding recommendation and plan activity state seed per 1,000 sq ft; the recommendation adds the total for the lawn when the profile records its size | No amounts | Seed per 1,000 sq ft is an agronomic amount, not a product label rate. A pre-emergent product rate is never stated: the label governs. |
| Seed total without a lawn size | Per-1,000 sq ft amount only | Total for a default 5,000 sq ft lawn | A total for an assumed size reads as the owner's lawn and is wrong for most; the feeding advice already withholds it (RULE-ENG-009). |

## Open Questions & Future Decisions

### Resolved
1. ✅ The windows use measured station data, not a modeled grid. Open-Meteo stays out of the window dating; a forecast-only swap is parked (`docs/plans/open-meteo-forecast.md`).
2. ✅ Seed vs. pre-emergent has no toggle or setting; the application log decides (owner decision).
3. ✅ The seed-amount note is intended: seed per 1,000 sq ft is an agronomic amount. Any pre-emergent product rate in advice would be a divergence.

### Deferred
1. **A single extreme day can confirm a crossing.** The trailing 5-day mean carries one outlier for 5 days, so `detect` can confirm it (`series.rs:94-157`). The owner has not decided whether to require the raw daily value to hold as well.
2. **A composite can hide an overdue trigger.** When one option is overdue (undated) and the other has a future typical date, the composite shows the other date as its estimate; the undated option is filtered out (`evaluate.rs:174-195`).
3. **GDD coverage short-circuit.** `gdd_reached` returns `None` at the first day the target is met if coverage is below 80 %, and does not look at later days (`climatology.rs:91-93`). The boundary then falls back to typical and can go overdue for the rest of the season.
4. **No single-flight on a memo miss.** Concurrent cache misses each read the 15-year record (`api/timing.rs:47-55`).
5. **Both applications logged.** When Overseed and PreEmergent are both logged in one fall, both windows read Done, since Done is checked before Blocked (`mod.rs:171-175`). The contradiction is not surfaced.
6. **Latent: the page and the plan propagate `get_or_refresh` errors** (`api/timing.rs::lake_and_forecast`, line 66). The refresh logs and swallows every source failure today (`logic/data_sync.rs`), so a failed forecast already reaches the windows as no forecast with its data note (TIME-ENG-045). An error the refresh might return in future would fail the endpoint and drop the timing activities from the seasonal plan, though the lake read is independent.
7. **Technical debt:**
    - `logic/timing/evaluate.rs` is 353 lines (over 300).
    - `companions::aeration` is 83 lines.
    - The default export of `SoilSeasonChart.tsx` is about 145 lines.
    - The doc comment on `fall_freeze_history` says "Oct–Dec data", but the check runs from Sep 15 to the freeze.
    - The `BoundaryView.label` example says "falls" while labels say "cools".
8. **Stale method doc** (`docs/timing-windows.md`):
    - It says "warm-season and mixed lawns get the two pre-emergent windows only" (line 93). Mixed is cool-season and gets the seeding windows; the Mixed wording needs fixing.
    - Its log table lists PreEmergent Jul 1 – Dec 31 as blocking dormant seeding. The code extends that span to Feb 28 of the following year (`log.rs:25`).
9. Fall fertilizer, herbicide and winterizer plan activities still time off 10 cm crossings in the seasonal-plan component. Whether they should hang off windows is undecided.
11. **Promotion trigger.** This stays one leaf while the consumers (feed, plan, companions) are fixed mappings from window state and change together with the engine. Promote it to a sub-HLD when more consumers hang off the windows (item 10) or a second forecast source (`docs/plans/open-meteo-forecast.md`) is adopted. The split is `window-engine` (TIME-ENG + TIME-LOG) and `window-consumers` (TIME-FEED + TIME-PLAN), both under "one answer everywhere"; the UI stays with whichever owns the response it renders.

## References

- `docs/timing-windows.md`: full method, thresholds, caveats, and the deliberate differences from lawn-answers.com
- `docs/high-level-design.md`: tenets and the component map
- `docs/agronomy-methods.md`: the cross-feature thresholds summary
- `backend/src/logic/timing/`, `backend/src/api/timing.rs`, `backend/src/models/timing.rs`, `backend/src/datasources/weather/climatology.rs`
- `frontend/src/pages/Timing.tsx`, `frontend/src/components/timing/`, `frontend/src/types/timing.ts`
- Sibling LLDs: weather-data (WX), data-sync (SYNC), recommendation-feed (FEED), seasonal-plan (PLAN), applications (APP), product-inventory (INV), agronomic-rules (RULE, owns `broadleaf_fall`), mcp-server (MCP), web-ui (UI)
- NOAA USCRN PA Avondale (WBANNO 3761); lawn-answers.com SeedTiming / SpringPreEm / FallPreEm
