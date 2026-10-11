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
   (gold 10 cm and air daily means). Fewer than 14 pairs is not enough data (503, below).
2. **Current soil.** `current.soil_temp_10_f` from data sync's `get_or_refresh` (this takes the
   sync write lock). If it is missing, that is not enough data (503).
3. **Forecast air.** For each `full_days()` forecast day, `(high + low) / 2`. If there are none,
   that is a forecast outage (503 naming the weather forecast).
4. **Fit** (`fit_model`). Least-squares `soil(d) = slope × air(d − lag) + intercept` for lag
   0–7 days, pairing each day's soil with the air of the calendar day `lag` days earlier — by
   date, so a missing day in the record drops its pairs instead of shifting every later one.
   Each lag needs ≥ 10 date-aligned pairs. The lag with the best R² is kept, and an R² below
   0.3 means no model (not enough data, 503).
5. **Air series** (`continuous_air`). The pairs' station air, then the forecast air, as one
   daily series: station values win for the same day, and any hole of up to 7 days between
   or inside them (the lake trails real time by a day or more; the forecast starts today) is
   linearly interpolated.
6. **Predict** (`anchored_outlook` over `predict_soil_temps`). For every day after the last
   observed soil day that the air series can drive, the driver is the air `lag` days earlier.
   **A day whose driver is missing is skipped.** Every prediction is shifted by the model's
   error on the last observed day (observed − fitted), so the outlook continues from the
   measured soil rather than jumping to the fitted line, and values are rounded to 0.1 °F.
   Each prediction carries a confidence (below). Predictions start the day after the lake's
   last day, so the first one or two can be dated before today.
7. **Crossings** (`predict_threshold_crossings`), against "today", walked from the live 10 cm
   reading through the predictions dated after today — days the lake has not reported yet but
   that are already past are history to the live reading.

"Today" and "days out" are the lawn's local date (parent decision): the handler passes
`state.clock.today()` (the app's `Clock`, from `TZ`) into `build_forecast(.., today)`, which
dates crossings from it and hands it to `predict_soil_temps(.., today)` for each confidence's
days out. Forecast days are the same local dates, so a crossing tomorrow is one day out.

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
(last prediction − current) over the forecast span. When that date falls after the last
prediction, the approach gets **no date** (`estimated_crossing_date: null`,
`beyond_forecast: true`) and is described as "beyond the forecast"; its extrapolated
`days_until_crossing` is kept only for sorting. Crossings are sorted by
`days_until_crossing`.

## Contract with Timing Windows

`fit_model(pairs)`, `continuous_air(observed, forecast)` and
`anchored_outlook(model, air, last_soil, today)` (over `predict_soil_temps`) are pure functions
that both outlooks call; timing windows uses them for its 5 cm outlook
(`backend/src/logic/timing/mod.rs`). The contract:

- `fit_model` takes `(date, air_f, soil_f)` rows in ascending order and returns `None` with
  fewer than 10 aligned pairs at every lag or a best R² below 0.3. The 14-pair minimum is the
  10 cm endpoint's gate, not the function's.
- `predict_soil_temps` returns one prediction per outlook day whose lagged driver is found in
  the history series or the outlook series, in outlook order, skipping the rest.
- `continuous_air` joins `(date, air_f)` station and forecast series, station winning, holes of
  up to 7 days (`MAX_AIR_GAP_DAYS`) interpolated, longer ones left open.
- `anchored_outlook` predicts every day after `last_soil`'s date that `air` can drive, shifted
  by the model's error on that day.
- The caller supplies its own series and depth. Timing windows fits on 5 cm over its last 30
  days (`docs/timing-windows.md`). It ignores the per-prediction confidence, so it passes its
  last observed day as `today`.

Changing these signatures or the alignment semantics changes both outlooks, so a change here is
a cross-segment change that pauses for timing windows.

## API Endpoint

| Method | Path | Behavior | Errors |
|---|---|---|---|
| GET | `/api/v1/soil-temp-forecast` | The pipeline above | 503 `not_enough_data(Source::SoilOutlook)` for insufficient pairs, no current soil reading, a poor fit, or no prediction made: "Not enough recent station data to project soil temperature. Try again once the station has reported a few more days."; 503 forecast outage when there is no full forecast day; 503 lake-not-configured when the lake client is missing; 503 lake-unreadable (detail logged) for a lake error |

The MCP server exposes it as `soil_temp_outlook`.

| Failure | Effect |
|---|---|
| Lake unreadable | 503 naming the weather lake |
| OpenWeatherMap down | 503 once data sync's forecast reuse window expires |
| Under 14 daily pairs, poor fit, or every day skipped | 503 with the not-enough-data sentence. The Environmental page hides the section; the widget is not rendered |

## UI Owned

**`PredictionChart`** (`frontend/src/components/PredictionChart.tsx`, on the Environmental
page). It plots the last 7 days of the page's `/historical` 10 cm series as a solid line
(`lastDays`, whatever range the trend selector shows), then the predictions dashed at local
noon, joined by a bridge point. It draws reference
lines only at 60 °F (Grub Control) and 75 °F (Heat Stress). Below it are crossing cards
(direction arrow, temperature, `~N days (date)` or "beyond the forecast" (`crossingWhen`), a
confidence badge colored by
`PREDICTION_CONFIDENCE_COLORS` with its label) and a model line (R², lag, quality).

**`SoilTempForecastWidget`** (`frontend/src/components/SoilTempForecastWidget.tsx`, rendered by
the dashboard). It keeps crossings more than 0 days out. A falling crossing counts only if the
last prediction is still below the threshold, so a transient dip is not a headline. The nearest
remaining crossing is the headline (`~Nd until soil reaches / drops below X°F`, threshold name,
estimated date, confidence badge; an approach beyond the forecast reads "Later" and "Beyond the
forecast" instead of a count and a date), and up to two more follow. When none remain it shows
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
| Crossing walk | Predictions dated after today, from the live reading | Every prediction, including lake-lag days | The live reading is newer than the lake's last day; estimates for days already past are history to it, and walking through them could announce a crossing that has already happened. |
| Prediction chart span | Last 7 days, whatever the trend range | The trend selector's range | The chart is about the next few days; at 30d or 90d a weeks-long history flattens the outlook. |
| Outlook unavailable for lack of data | 503 with a plain message | 400 `InvalidData` | Nothing was wrong with the request; the owner needs to know the data is thin (HLD human-readable errors). |

## Open Questions & Future Decisions

### Resolved

1. ✅ The 10 cm outlook announces only direction-aware 10 cm thresholds. Pre-emergent and
   seeding belong to the 5 cm timing windows.

### Deferred

1. **Technical debt.** `backend/src/logic/soil_temp_prediction.rs` is 337 lines without its
   tests (limit 300). Functions over 50 lines: `predict_threshold_crossings` (~90),
   `get_soil_temp_forecast` (~72).

## References

- Parent: `docs/intent/weather-data/weather-data-design.md`.
- `docs/intent/weather-data/weather-sources/`: `fetch_daily_paired_averages`, `full_days()`.
- `docs/intent/timing-windows/` and `docs/timing-windows.md`: the 5 cm outlook built on
  `fit_model` / `predict_soil_temps`.
- `docs/agronomy-methods.md`: the outlook's thresholds (served over MCP; keep in sync).
- Code: `backend/src/logic/soil_temp_prediction.rs`, `backend/src/api/soil_temp_prediction.rs`,
  `backend/src/models/soil_temp_prediction.rs`, `frontend/src/components/PredictionChart.tsx`,
  `frontend/src/components/SoilTempForecastWidget.tsx`.
