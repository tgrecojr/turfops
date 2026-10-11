---
parent: weather-data
prefix: WX-OUTLOOK
---

# Soil-Temperature Outlook

## Context and Design Philosophy

The station measures soil temperature but cannot say where it is heading. This leaf projects
10 cm soil temperature over the forecast days from a lagged regression on the station's own
recent air and soil record, and names the agronomic 10 cm thresholds the projection crosses. It
owns `GET /api/v1/soil-temp-forecast`, the `PredictionChart` on the Environmental page and the
dashboard `SoilTempForecastWidget`, and it publishes the regression functions that timing
windows reuses for its 5 cm outlook.

Principles:

- **One answer everywhere.** The 10 cm outlook announces only questions no other component
  answers. Pre-emergent, crabgrass and seeding belong to the 5 cm timing windows, so they have
  no 10 cm threshold here.
- **Measured over modeled.** The model is fitted to the station's last 30 days, not taken from a
  modeled soil product.
- **Withhold rather than invent.** No model is published when the fit is poor or the data is too
  thin. A day whose air driver is unknown is skipped rather than guessed.

## Pipeline

`backend/src/api/soil_temp_prediction.rs` (handler) and `backend/src/logic/soil_temp_prediction.rs`
(pure logic):

1. **Training data.** Weather sources' `fetch_daily_paired_averages` over the last 30 days
   (gold 10 cm and air daily means). Fewer than 14 pairs returns 400.
2. **Current soil.** `current.soil_temp_10_f` from data sync's `get_or_refresh` (this takes the
   sync write lock). If it is missing, the endpoint returns 400.
3. **Forecast air.** For each `full_days()` forecast day, `(high + low) / 2`. If there are none,
   it returns 503.
4. **Fit** (`fit_model`). Least-squares `soil[i + lag] = slope × air[i] + intercept` for lag
   0–7 days, aligned by array position. Each lag needs ≥ 10 aligned pairs. The lag with the best
   R² is kept, and an R² below 0.3 means no model (400).
5. **Predict** (`predict_soil_temps`). For each forecast day, the driver is the air temperature
   `lag` days earlier: the observed pair series first, then forecast air. With lag 0 the day's
   own forecast is used. **A day whose driver is in neither series is skipped.** Values are
   rounded to 0.1 °F. Each prediction carries a confidence (below).
6. **Crossings** (`predict_threshold_crossings`), against "today".

"Today" and "days out" are the server's local date (parent decision). The code uses the UTC
date (Deferred 1).

| Confidence | Rule (`compute_confidence`) |
|---|---|
| High | R² ≥ 0.7 and ≤ 2 days out |
| Medium | R² ≥ 0.5 and ≤ 3 days out |
| Low | otherwise |

Model quality labels: R² ≥ 0.7 is Good, ≥ 0.5 is Fair, below that Poor. The response
(`SoilTempForecast`, `backend/src/models/soil_temp_prediction.rs`) carries `predictions`,
`threshold_crossings`, `model_info` (R², lag, training days, quality; coefficients are not
exposed) and `generated_at`.

## Thresholds and Crossings

`THRESHOLDS`, each announced only in its meaningful direction:

| Name | 10 cm °F | Direction |
|---|---|---|
| Winterizer / Dormancy | 45 | falling |
| Grub Control Window | 60 | rising |
| Active Growth Peak | 65 | both |
| Heat Stress Risk | 75 | rising |

Crossing detection walks from the current reading through the predictions. The first step that
moves across a threshold is the crossing. A crossing in the wrong direction ends the search for
that threshold without announcing anything. If nothing crosses but the soil is below a
non-falling threshold, within 5 °F of it, and the last prediction is warmer than now, an
**approaching** crossing is added: Low confidence, Rising, dated by linear extrapolation of
(last prediction − current) over the forecast span. Crossings are sorted by
`days_until_crossing`.

## Contract with Timing Windows

`fit_model(pairs)` and `predict_soil_temps(model, history_air, outlook_air)` are pure functions
that timing windows calls for its 5 cm outlook (`backend/src/logic/timing/mod.rs`). The
contract:

- `fit_model` takes `(date, air_f, soil_f)` rows in ascending order and returns `None` with
  fewer than 10 aligned pairs at every lag or a best R² below 0.3. The 14-pair minimum is the
  10 cm endpoint's gate, not the function's.
- `predict_soil_temps` returns one prediction per outlook day whose lagged driver is found in
  the history series or the outlook series, in outlook order, skipping the rest.
- The caller supplies its own series. Timing windows refits on 5 cm, joins station and forecast
  air with the gap interpolated, and anchors to the last observed value
  (`docs/timing-windows.md`); none of that lives here. It ignores the per-prediction confidence.

Changing these signatures or the alignment semantics changes both outlooks, so a change here is
a cross-segment change that pauses for timing windows.

## API Endpoint

| Method | Path | Behavior | Errors |
|---|---|---|---|
| GET | `/api/v1/soil-temp-forecast` | The pipeline above | 400 for insufficient pairs, no current soil reading, a poor fit, or no prediction made; 503 when there is no forecast or the lake client is missing; 500 for a lake error |

The MCP server exposes it as `soil_temp_outlook`.

| Failure | Effect |
|---|---|
| Lake unreadable | 500 |
| OpenWeatherMap down | 503 once data sync's forecast reuse window expires |
| Under 14 daily pairs, poor fit, or every day skipped | 400. The Environmental page hides the section; the widget is not rendered |

## UI Owned

**`PredictionChart`** (`frontend/src/components/PredictionChart.tsx`, on the Environmental
page). It plots the last 168 points of the page's selected `/historical` 10 cm series as a solid
line, then the predictions dashed at local noon, joined by a bridge point. It draws reference
lines only at 60 °F (Grub Control) and 75 °F (Heat Stress). Below it are crossing cards
(direction arrow, temperature, `~N days (date)`, a confidence badge colored by
`PREDICTION_CONFIDENCE_COLORS` with its label) and a model line (R², lag, quality).

**`SoilTempForecastWidget`** (`frontend/src/components/SoilTempForecastWidget.tsx`, rendered by
the dashboard). It keeps crossings more than 0 days out. A falling crossing counts only if the
last prediction is still below the threshold, so a transient dip is not a headline. The nearest
remaining crossing is the headline (`~Nd until soil reaches / drops below X°F`, threshold name,
estimated date, confidence badge), and up to two more follow. When none remain it shows
"Current: X°F — No threshold crossings predicted" (or "Prediction data unavailable").

## Decisions & Alternatives

| Decision | Chosen | Alternatives Considered | Rationale |
|---|---|---|---|
| 10 cm outlook thresholds | 45 falling, 60 rising, 65 both, 75 rising. No pre-emergent, crabgrass or seeding threshold | The full 10 cm milestone set in both directions | One answer everywhere: those milestones are the 5 cm timing windows' job, and a second 10 cm answer (e.g. a "pre-emergent window" while soil falls through 50 °F in October) contradicts them (HLD tenet; `logic/soil_temp_prediction.rs`). |
| Outlook model | Lagged linear air → soil regression, lag 0–7 chosen by R², 30-day window, ≥ 14 pairs, R² ≥ 0.3 | Physical heat-flow model; a modeled soil forecast product | A simple fit to the station's own recent record. Modeled soil products run days early (owner's Open-Meteo comparison). Parameter values [inferred]. |
| Forecast air driver | Mean of high and low on full forecast days only | Every daily bucket | A partial first or last day would enter the regression as a full-day mean (comment in the handler; `full_days()` contract). |
| Reuse by timing windows | One set of pure functions shared by both outlooks: the fit, the prediction, the joined air series (station air preferred, gaps ≤ 7 days interpolated) and the anchor to the last observation | A separate 5 cm regression; anchoring and gap-filling only in the 5 cm outlook | One method for air → soil projection, so the 10 cm and 5 cm outlooks cannot disagree in method; each caller still supplies its own soil depth and series. Without the anchor, model bias alone can produce a day-one crossing. |
| Lag alignment | By calendar date | By list position | A missing day in the training window must not silently shift the lag. |
| Approaching crossing past the forecast | No date, "beyond the forecast" | Extrapolate without limit | Withhold rather than invent: the data supports no date past the forecast. |
| Outlook unavailable for lack of data | 503 with a plain message | 400 `InvalidData` | Nothing was wrong with the request; the owner needs to know the data is thin (HLD human-readable errors). |

## Open Questions & Future Decisions

### Resolved

1. ✅ The 10 cm outlook announces only direction-aware 10 cm thresholds. Pre-emergent and
   seeding belong to the 5 cm timing windows.

### Deferred

1. **Bug: "today" is the UTC date.** `build_forecast` dates crossings from
   `Utc::now().date_naive()` (`backend/src/logic/soil_temp_prediction.rs:299`), and
   `predict_soil_temps` computes each confidence's "days out" the same way (`:140`), while the
   forecast days are local dates. Every evening after UTC midnight `days_until_crossing` and the
   confidence are a day off (a tomorrow crossing reads as today and drops out of the widget's
   `> 0` filter). Intent is the server's local date. `predict_soil_temps` is shared, so the fix
   touches timing windows' call site too (it ignores the confidence).
2. **Divergence: not anchored, lag hole not filled** (WX-OUTLOOK-MODEL-008). Intent: the 10 cm outlook uses the same joined air series and anchoring as the 5 cm outlook, from shared functions. Unlike the 5 cm outlook, the 10 cm outlook is not
   anchored to the last observation and does not interpolate between the lake's last day and
   the first forecast day. A forecast day whose lagged driver falls in that hole is skipped
   (`soil_temp_prediction.rs:114-134`). If every day is skipped, `build_forecast` returns `None`
   and the endpoint reports "poor data correlation" (`api/soil_temp_prediction.rs:76-78`), which
   is misleading.
3. **First crossing step compares unlike values** (resolved by anchoring, WX-OUTLOOK-MODEL-008). Detection starts from the live 10 cm reading
   and steps to unanchored model output (`soil_temp_prediction.rs:212-217`), so model bias alone
   can produce a day-1 crossing.
4. **Divergence: lag aligned by index** (WX-OUTLOOK-MODEL-001). Intent: pairs aligned by date. `fit_model` pairs `air[i]` with `soil[i + lag]` by array position
   (`soil_temp_prediction.rs:68-71`), so a missing gold day in the 30-day window silently shifts
   the lag. Timing windows' 5 cm fit inherits this.
5. **Divergence: approaching date uncapped** (WX-OUTLOOK-CROSS-005). Intent: no date past the forecast span. The "approaching" crossing is extrapolated linearly with no cap
   past the forecast span (`soil_temp_prediction.rs:252-276`), which can date a crossing the data
   does not support.
6. **Divergence: error shapes** (WX-OUTLOOK-API-003). Intent: 503 with a plain-language message. Insufficient pairs, no current soil reading and a poor fit return 400
   `InvalidData` (`api/soil_temp_prediction.rs:28-32`, `:41-43`, `:76-78`), although the client
   sent nothing wrong.
7. **Divergence: chart span follows the trend selector** (WX-OUTLOOK-UI-004). Intent: always the last 7 days. `PredictionChart` takes its "recent" line from the
   Environmental page's currently selected `/historical` range (last 168 points), so at 30d or
   90d the line covers weeks, not the last 7 days (`PredictionChart.tsx:33`).
8. **Technical debt.** `backend/src/logic/soil_temp_prediction.rs` is 566 lines (about 310
   without tests; limit 300). Functions over 50 lines: `predict_threshold_crossings` (~83),
   `get_soil_temp_forecast` (~71).

## References

- Parent: `docs/intent/weather-data/weather-data-design.md`.
- `docs/intent/weather-data/weather-sources/`: `fetch_daily_paired_averages`, `full_days()`.
- `docs/intent/timing-windows/` and `docs/timing-windows.md`: the 5 cm outlook built on
  `fit_model` / `predict_soil_temps`.
- `docs/agronomy-methods.md`: the outlook's thresholds (served over MCP; keep in sync).
- Code: `backend/src/logic/soil_temp_prediction.rs`, `backend/src/api/soil_temp_prediction.rs`,
  `backend/src/models/soil_temp_prediction.rs`, `frontend/src/components/PredictionChart.tsx`,
  `frontend/src/components/SoilTempForecastWidget.tsx`.
