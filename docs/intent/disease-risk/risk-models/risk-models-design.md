---
parent: disease-risk
prefix: DIS-MODEL
---

# Disease Risk Models

## Context and Design Philosophy

Risk-models turns measured station weather and the forecast into one tier per disease: Low, Moderate, High or Severe. It produces a headline tier for "now" and a daily tier for each day of the past week and the outlook. The headline tier is the contract the parent defines: management, the feed and the UI all read it, and none of them reads weather. This leaf also defines `today` for the whole subtree.

Guiding principles:

- **Published models, native scores.** Each disease keeps the score its source defines. Experimental indices are labeled `validated: false` and shown as experimental.
- **Measured over modeled.** Observed days come from the station's silver hourly data. The forecast fills only the hours the lake has not reported yet and the days ahead. Lake lag is disclosed in `data_notes` rather than hidden.
- **Withhold rather than invent.** A day that lacks the hours a model depends on cannot be the headline. When no recent day qualifies, the disease is left out.
- **Weather drives the daily series; history only raises the headline.** Lawn history multiplies susceptibility. It never creates risk out of unfavorable weather, and it never changes a daily bar.

| Owned | Path |
|---|---|
| Window constants, usable-day rule, trailing mean, headline scoring, amplifier, `assess_all` | `backend/src/logic/disease/mod.rs` |
| The five models | `logic/disease/brown_patch.rs`, `dollar_spot.rs`, `pythium.rs`, `gray_leaf_spot.rs`, `red_thread.rs` |
| Forecast days and the lake + forecast join | `logic/disease/weather_days.rs` |
| History fields of `DiseaseContext` (days since overseeding / fertilizer) | `logic/disease/history.rs` |
| Data types (`DailyWeather`, `DailyRisk`, `DiseaseRisk`, `RiskScale`, `RiskFactor`, `Methodology`, `DiseaseRiskResponse`) | `backend/src/models/disease.rs` |
| `GET /disease-risk` assembly: `compute`, `data_notes` | `backend/src/api/disease_risk.rs` |
| Query shape of the per-day disease inputs | `backend/src/datasources/weather/disease.rs` (weather-data owns the client) |

## Today and Day Boundaries

**Intent.** `today` is the server's local date (`chrono::Local`, governed by `TZ`, which deployment sets to the lawn's timezone). Observed lake hours keep the lake's `obs_date_local` (the station's civil date in America/New_York, daylight-saving aware), which is the calendar `TZ` gives; forecast points are bucketed into those same local days. Every date in the subtree (the headline day, the display window, residual windows, the outlook, the rotation year, the 365-day log read) is measured from this `today`.

**Current state (divergence).** The code runs a second clock, derived from the lake (Deferred 6):

- `today` is UTC now plus the lake's UTC offset (`api/disease_risk.rs:63`). The offset is read from the station's newest silver row (`datasources/weather/disease.rs:74-84`).
- Observed hours are grouped by the lake's own `obs_date_local` (`datasources/weather/disease.rs:36-51`), as intended. Forecast points are bucketed with the lake offset of the newest silver row (`weather_days.rs:24-29`, called at `api/disease_risk.rs:67`), which is wrong for forecast hours that fall across a daylight-saving change.
- The fetch start is anchored on the UTC date, one day early so a date mismatch cannot cut the lookback short (`api/disease_risk.rs:60-61`).

This is harmless while `TZ` matches the station's offset, but it is not the designed clock.

## Daily Weather Series

`DailyWeather` (`models/disease.rs`) describes one local day in **°C**, because the published models are specified in °C. It is never stored. Its fields:

- mean, min and max temperature, and mean RH;
- `hours_rh90`, and `leaf_wetness_hours` (hours with RH ≥ 90 % or any precipitation);
- precipitation, and peak dew point (Magnus);
- `hours_covered` and `is_forecast`;
- `covers_dawn` and `covers_afternoon`.

The lake query skips hours with no air temperature or RH, so `hours_covered` counts usable hours only.

**Window** (`logic/disease/mod.rs`): the fetch covers `LOOKBACK_DAYS` = 12 observed days, so trailing windows are already full on the first displayed day. The display shows `HISTORY_DAYS` = 6 days before today, today, and `FORECAST_DAYS` = 4 days after, for at most 11 days.

**Forecast days** (`weather_days::forecast_days`): 3-hourly points are bucketed into local days, and each point stands for 3 h, so `hours_covered = 3 × points`. A point counts toward `hours_rh90` when RH ≥ 90 %, and toward leaf wetness when RH ≥ 90 % or precipitation > 0. Forecast temperatures are converted from °F.

**Join** (`weather_days::build_series`):

- A day with 24 observed hours wins outright.
- A partly observed day (usually today, because the lake lags) is merged with that day's forecast. The merge weights means by hours, takes the min of minimums and the max of maximums, sums hours and precipitation, caps `hours_covered` at 24 and ORs the flags.
- Forecast-only days are appended.

The result is sorted ascending by date. The join assumes the lake and the forecast never cover the same hours (Deferred 2).

**Usable day** (`is_usable`): a day is usable when all three of these hold:

- `hours_covered ≥ 12`;
- `covers_dawn` (some local hour from 03 to 08);
- `covers_afternoon` (some local hour from 13 to 17).

**Trailing mean** (`trailing_mean`): averages a field over the usable days in a window ending at a given day. That day always counts, even when it is partial. The result is `None` when fewer than `min_days` days contribute.

## Headline and Fallback

`score_window` scores every day in the display window that has a value. Each becomes a `DailyRisk { date, value, tier, is_forecast, partial }`, where `partial = !is_usable`.

The headline is the **latest usable day on or before today**. That is today once enough of it is covered, otherwise the most recent complete day. The search reaches back no further than the start of the display window, 6 days. With no usable day in that span, the model returns `None` and the disease is left out of the response (Deferred 5). `DiseaseRisk.as_of` names the headline day.

## Risk Models

| Disease (slug) | Model · `validated` | Daily value | Moderate | High | Severe |
|---|---|---|---|---|---|
| Brown patch (`brown-patch`) | Fidanza-Dernoeden E-index · true | `E = −21.5 + 0.15·RH + 1.4·Tmin − 0.033·Tmin²` (that day) | ≥ 0 | ≥ 5 (Vincelli revised) | ≥ 6 (original Fidanza) |
| Dollar spot (`dollar-spot`) | Smith-Kerns logistic · true | 5-day trailing means of temp and RH (≥ 3 usable days); `logit = −11.4041 + 0.0894·RH + 0.1932·T`; 0 % outside 10–35 °C | ≥ 10 % | ≥ 20 % (published action threshold) | ≥ 40 % |
| Pythium blight (`pythium-blight`) | Nutter-criteria threshold score · false | +1 max ≥ 30 °C, +1 min ≥ 20 °C, +1/+2/+3 at ≥ 6/10/14 h RH ≥ 90 %. Capped at 1 when no heat criterion is met. | ≥ 2 | ≥ 3 | ≥ 4 |
| Gray leaf spot (`gray-leaf-spot`) | Suitability index · false | 100 × temp trapezoid (18/25/30/35 °C) × wetness ramp (6→16 h); 0 outside Jul–Oct; 3-day trailing mean | ≥ 25 | ≥ 50 | ≥ 75 |
| Red thread (`red-thread`) | Suitability index · false | 100 × temp trapezoid (4/15/22/27 °C) × wetness ramp (4→12 h, floored at 0.5 on a ≥ 2.5 mm rain day); 5-day trailing mean | ≥ 25 | ≥ 50 | ≥ 75 |

`RiskScale` carries each model's gauge bounds, unit and three tier floors, and `tier_for` maps a value to a tier.

Every model also produces:

- a list of `RiskFactor`s (label, value, Unfavorable / Marginal / Favorable, note);
- a one-sentence summary;
- a `Methodology` (model name, citation, `validated`, summary, steps).

The UI and the feed show these unchanged. Each model calls management with its disease, headline tier, daily tiers, `today` and the context, and embeds the returned plan in its `DiseaseRisk`.

Pythium's peak dew point is shown as a factor but not scored (`pythium.rs`). The tier thresholds were sanity-checked against five seasons of station data, and any retuning repeats that backtest.

## History Amplifiers

`history::context_from_history(apps, today)` reads **lawn** applications only. It ignores plant applications (`plant_id` set) and future-dated entries. It sets `days_since_overseed` and `days_since_fertilizer` from the latest `Overseed` and `Fertilizer` entries. Management owns the fungicide fields of the same context.

| Amplifier | Condition | Effect |
|---|---|---|
| Seedlings → gray leaf spot | `Overseed` ≤ 60 days ago | Headline tier raised one step |
| Nitrogen-hungry → red thread (`red_thread::nitrogen_hungry`) | No feeding (a turf `Fertilizer` with N unrecorded or above 0, the agronomic rules' definition) logged, or the last one > 60 days ago | Headline tier raised one step |

`amplify` raises a tier only when it is already ≥ Moderate, saturating at Severe. Only the headline is raised: `daily` stays weather-only, so its bars agree with the scale's thresholds. When a raise happens, `tier_note` reads "Raised from <tier> — …". The raised tier is the one handed to management and shown everywhere.

## Response and Data Notes

`GET /api/v1/disease-risk` returns `DiseaseRiskResponse { generated_at, today, station, diseases[], data_notes[] }`. It responds 404 with no lawn profile and 503 when the lake is not configured. A lake, sync or DB error is returned as an error. `compute` builds the series and the context, runs `assess_all` and assembles `data_notes`. It returns the context alongside the response so the feed bridge (management) reuses the same run.

`data_notes` (`api/disease_risk.rs:119-148`) are plain-language caveats about the inputs:

- no recent station observations;
- observations end more than a day before today, so more recent days rely on the forecast;
- today is not covered, so current risk reflects the most recent complete day;
- no forecast, so no outlook days are shown.

| Condition | Page behavior |
|---|---|
| Lake unconfigured or the read fails | Error response (feed behavior: see management) |
| Forecast missing | Observed days only, with a note; no outlook days |
| Lake lagging | Headline falls back to the last usable day (≤ 6 days back), with a note |
| No usable day in the window | That disease is left out. If all are left out, the list is empty and the first note explains why. |

## Decisions & Alternatives

| Decision | Chosen | Alternatives Considered | Rationale |
|---|---|---|---|
| Diseases covered | Brown patch, dollar spot, Pythium blight, gray leaf spot, red thread | Adding large patch, anthracnose | Large patch is a warm-season disease and anthracnose is a disease of Poa and bentgrass greens. Neither fits a tall-fescue-type lawn in Zone 7a. |
| Dollar spot model | Published Smith-Kerns (5-day moving averages, 20 % action threshold), checked against the UW Turfgrass Diagnostic Lab worked example | The reference site's DSV-style variant | The reference variant is a homebrew that pegs at 100 %. The published model is validated, including at a Pennsylvania site. |
| Same-day data gap | Lake + OWM join, falling back to the last usable day, with a data note | Open-Meteo hourly past + forecast | A new external dependency needs the owner's sign-off. Measured station data is preferred, and its lag is disclosed. |
| Usable day | ≥ 12 h **and** dawn (03–08) **and** afternoon (13–17) hours | Hours alone | [inferred] A forecast-only afternoon has 12 h but no dawn low or humid night, which drive most scores. A day that ends at noon has no high. Both spans are wider than the 3-hour forecast step, so a covered span always contains a point. |
| Headline day | Latest usable day ≤ today, at most 6 days back, else omit | Always today; any day in the lookback | [inferred] A partial today would understate or overstate risk. Older than the display window is no longer "current". |
| Amplifier scope | Raise only the headline tier; the daily series stays weather-only | Amplify every day | Lawn history raises only the headline tier (`docs/agronomy-methods.md`). The bars must match the scale's thresholds. |
| Amplifier floor | Raise only at ≥ Moderate | Raise from Low too | [inferred] History is a susceptibility multiplier. It should not create risk when the weather is unfavorable (`mod.rs` `amplify`). |
| Pythium dew point | Shown for context, not scored | Score it as a criterion | [inferred] The station backtest found it nearly redundant with a warm night, and it inflated High/Severe days (`pythium.rs`). |
| Merged partial day | Forecast points fill only hours the lake has not reported | Sum both sources and cap hours at 24 | Summing double-counts humid hours and inflates Pythium and gray leaf spot; capping only `hours_covered` hides it. |
| Data notes | Judge "today usable" with the headline's usable-day rule, and name any disease left out | Hours alone; silent omission | A note must explain every fallback and every missing disease, or the page reads as if the risk were Low. |
| Model temperature unit | °C inside `DailyWeather`, which is never stored | °F, as stored data is | [inferred] The published formulas are specified in °C. Values are converted to °F only for display. |
| Day boundary | Server-local date (`TZ`) for `today`; the lake's `obs_date_local` (the same Eastern, daylight-saving-aware calendar) for observed hours | UTC date; station-local `today` from the lake's offset; re-bucketing lake hours by `TZ` | Every agronomic date is a lawn-local calendar day, and one configured clock gives every component the same day (HLD). The code still uses the station-local clock (Deferred 6). |

## Open Questions & Future Decisions

### Resolved

1. ✅ Same-day data stays lake + OWM. Open-Meteo is parked until the owner signs off, and the lag is disclosed in `data_notes`.
2. ✅ The disease set is five diseases. Large patch and anthracnose are excluded (owner decision).

### Deferred

1. **Divergence: data note vs usable-day rule** (DIS-MODEL-API-004). `data_notes` judges "latest observed" and "today usable" on `hours_covered ≥ 12` alone (`api/disease_risk.rs:121-125, 134-143`). When today has 12+ hours but no dawn or afternoon hours, the headline falls back to an earlier day, but the "reflects the most recent complete day" note is not shown.
2. **Divergence: merge overlap** (DIS-MODEL-DAY-007). Intent: forecast points fill only hours the lake has not reported. `build_series` assumes the lake and the forecast never cover the same hours. If they do, `merge_day` sums them with no de-duplication. Only `hours_covered` is capped at 24, so `hours_rh90` and `leaf_wetness_hours` can exceed 24 h and double-count (`weather_days.rs:70-88, 97-101`). A merged day also reports `is_forecast = true`.
3. **Recomputed per request.** The dashboard triggers two full computations, one through the feed and one through the widget, each with a lake read (`api/disease_risk.rs:28-41, 47-116`). There is no memo.
4. **Leaf wetness is a proxy.** It is approximated as RH ≥ 90 % or precipitation. Unused lake columns (`solar_rad_wm2`, `surface_temp_c`) could improve it.
5. **Divergence: no usable day → silent omission** (DIS-MODEL-API-005). Intent: a data note names the omitted disease. When no day in the 6-day lookback is usable, `score_window` returns `None` and the disease drops out of the response with no note that names it (`logic/disease/mod.rs:120`). The list shows the remaining diseases. A link to the missing one shows the generic "No risk data for …" (`pages/DiseaseRisk.tsx`).
6. **Station-local clock** (DIS-MODEL-DAY-001, DIS-MODEL-DAY-003). `today` and forecast-day bucketing use the lake's UTC offset, not the server-local date (see Today and Day Boundaries; `api/disease_risk.rs:61-67`, `datasources/weather/disease.rs:36-51`, `weather_days.rs:24-29`). This `today` also drives management's residual windows, outlook and rotation year, and the 365-day log read (`api/disease_risk.rs:74-81`). When the station has no silver row at all, the offset defaults to 0 and `today` becomes the **UTC date** (`datasources/weather/disease.rs:81-84`).
7. **Tech debt** against the 50-line limit: `api::disease_risk::compute` (~70 lines), `brown_patch::factors` (~72), `pythium::factors` (~65).
8. **Divergence: nitrogen-hungry counts any Fertilizer** (DIS-MODEL-AMP-001). `days_since_fertilizer` takes the latest `Fertilizer` entry (`logic/disease/history.rs`), so a potash-only product resets it. Intent: only a feeding counts.

## References

- Parent: `docs/intent/disease-risk/disease-risk-design.md`; siblings `management/`, `disease-ui/`
- `docs/agronomy-methods.md` § Disease risk: owner-facing method, kept in sync with these thresholds
- Fidanza, Dernoeden & Grybauskas (1996), Phytopathology 86:385–390 (brown patch E-index)
- Smith, Kerns et al. (2018), PLOS ONE 13(3):e0194216 (dollar spot)
- Nutter, Cole & Schein (1983), Plant Disease 67:1126–1128 (Pythium criteria)
- Uddin, Serlemitsos & Viji (2003), Phytopathology 93:336–343 (gray leaf spot drivers)
- Weather data access (WX): `fetch_daily_disease_inputs`, the OWM forecast
