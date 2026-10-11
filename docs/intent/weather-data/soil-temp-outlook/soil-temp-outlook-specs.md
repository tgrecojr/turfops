# Soil-Temperature Outlook — EARS Specs

Design: [soil-temp-outlook-design.md](soil-temp-outlook-design.md). Parent: [weather-data](../weather-data-design.md). Prefix `WX-OUTLOOK`. Facets: `MODEL` (regression and predictions, including the contract timing windows reuses), `CROSS` (thresholds and crossings), `API` (`GET /soil-temp-forecast`), `UI` (`PredictionChart`, `SoilTempForecastWidget`).

The "10 cm outlook" is this leaf's projection of station 10 cm soil temperature over the full forecast days. "The regression" is the shared pair `fit_model` / `predict_soil_temps`.

## Regression and predictions

- [ ] **WX-OUTLOOK-MODEL-001**: `fit_model` shall fit `soil = slope × air(lagged) + intercept` by least squares for each lag from 0 to 7 days, pairing each day's soil with the air of the calendar day `lag` days earlier (by date, never by list position), require at least 10 date-aligned pairs at a lag, keep the lag with the best R², and return no model when no lag qualifies or the best R² is below 0.3.
- [x] **WX-OUTLOOK-MODEL-002**: `predict_soil_temps` shall return one prediction per outlook day whose lagged air driver exists in the history series or the outlook series, in outlook order, skipping a day whose driver is in neither, with values rounded to 0.1°F.
- [x] **WX-OUTLOOK-MODEL-003**: The regression shall stay a pair of pure functions over caller-supplied series, so the 10 cm outlook and the timing windows' 5 cm outlook share one method while each owns its own series.
- [x] **WX-OUTLOOK-MODEL-004**: The 10 cm outlook shall train on the lake's gold daily air and 10 cm soil means for the last 30 days and require at least 14 pairs.
- [x] **WX-OUTLOOK-MODEL-005**: The 10 cm outlook shall drive predictions with (high + low) / 2 of full forecast days only.
- [x] **WX-OUTLOOK-MODEL-006**: The 10 cm outlook shall give each prediction a confidence: High when R² ≥ 0.7 and at most 2 days out, Medium when R² ≥ 0.5 and at most 3 days out, otherwise Low; and label the model Good (R² ≥ 0.7), Fair (≥ 0.5) or Poor.
- [ ] **WX-OUTLOOK-MODEL-007**: The 10 cm outlook shall measure "today" and "days out" from the server's local date (`TZ`).
- [ ] **WX-OUTLOOK-MODEL-008**: The 10 cm outlook shall drive the regression with one continuous air series (station daily mean air preferred over forecast (high + low) / 2, gaps of up to 7 days linearly interpolated) and shift every prediction by the model's error on the last observed day, using the same shared functions as the timing windows' 5 cm outlook.

## Thresholds and crossings

- [x] **WX-OUTLOOK-CROSS-001**: The 10 cm outlook shall announce only these thresholds, each in its stated direction: Winterizer / Dormancy 45°F falling, Grub Control Window 60°F rising, Active Growth Peak 65°F either direction, Heat Stress Risk 75°F rising; it shall have no pre-emergent, crabgrass or seeding threshold.
- [x] **WX-OUTLOOK-CROSS-002**: When detecting crossings, the 10 cm outlook shall walk from the current reading through the predictions, take the first step across each threshold as its crossing, and end the search for a threshold without announcing it when the first step across it is in the wrong direction.
- [x] **WX-OUTLOOK-CROSS-003**: When no crossing is found but the soil is below a threshold that is not falling-only, within 5°F of it, and the last prediction is warmer than now, the 10 cm outlook shall add an approaching rising crossing at Low confidence, dated by linear extrapolation over the forecast span.
- [ ] **WX-OUTLOOK-CROSS-005**: When an approaching crossing's extrapolated date falls beyond the last forecast day, the 10 cm outlook shall give it no date and describe it as beyond the forecast.
- [x] **WX-OUTLOOK-CROSS-004**: The 10 cm outlook shall sort crossings by days until crossing.

## Endpoint

- [x] **WX-OUTLOOK-API-001**: When `GET /api/v1/soil-temp-forecast` succeeds, the system shall return the predictions, the threshold crossings, model information (R², lag, training days, quality, with no coefficients) and the generation time.
- [x] **WX-OUTLOOK-API-002**: If there is no forecast or no lake client, then `GET /api/v1/soil-temp-forecast` shall return 503.
- [ ] **WX-OUTLOOK-API-003**: If there are fewer than 14 training pairs, no current 10 cm reading, no usable fit, or no prediction, then `GET /api/v1/soil-temp-forecast` shall return 503 with a plain-language message (for example "Not enough recent station data to project soil temperature"), not a 400.

## UI

- [x] **WX-OUTLOOK-UI-001**: The prediction chart on the Environmental page shall plot recent 10 cm history as a solid line and the predictions dashed at local noon, joined by a bridge point, with reference lines at 60°F (Grub Control) and 75°F (Heat Stress), and below it crossing cards (direction arrow, temperature, "~N days (date)", a confidence badge with its label) and the model line (R², lag, quality).
- [x] **WX-OUTLOOK-UI-002**: The dashboard soil outlook widget shall consider crossings more than 0 days out, count a falling crossing only while the last prediction is still below the threshold, headline the nearest remaining crossing ("~Nd until soil reaches / drops below X°F", threshold name, estimated date, confidence badge) followed by up to two more, and otherwise show "Current: X°F — No threshold crossings predicted".
- [x] **WX-OUTLOOK-UI-003**: If the soil outlook is unavailable, then the Environmental page shall hide the prediction section and the dashboard shall not render the widget.
- [ ] **WX-OUTLOOK-UI-004**: The prediction chart shall plot the last 7 days of 10 cm history as its recent line, whatever range the Environmental page's trend selector shows.
