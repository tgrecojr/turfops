# Plan: Replace OpenWeatherMap with Open-Meteo for the forecast (DEFERRED)

**Status: not started — deliberately parked on 2026-09-21.** The owner wants to live with the
current implementation (after the functional-review fixes, PRs #212–#220) and see whether this
is needed or is change for its own sake. **Do not implement until one of the triggers in
"When to pull the trigger" is actually observed.** Two design decisions are still open
(see "Open decisions").

## Context

TurfOps has three weather inputs:

| Input | Source | Used for |
|---|---|---|
| Soil temp/moisture, air temp, RH, precip — **measured**, hourly since 2011 | NOAA USCRN PA Avondale (WBANNO 03761) via the data lake (silver hourly, gold daily) | Timing windows (5 cm), rules (10 cm), GDD, freeze dates, disease history, 15-yr climatology |
| Ambient temp/RH right now | Home Assistant patio sensor | Dashboard, fertilizer block |
| **Forecast** — 5 days, **3-hourly** | OpenWeatherMap (`datasources/openweathermap.rs`, needs `OWM_API_KEY`) | Rain delay, irrigation, heat stress, application window, disease risk "today + outlook", soil-temp outlooks |

This plan only concerns the third row. It came out of comparing TurfOps' data with the 11 free
APIs lawn-answers.com uses (catalog: project memory `reference_lawn_answers_datasources`).

### What was measured (2026-09-21)

Open-Meteo Archive (ERA5) at the station's coordinates vs. 14 years of measured USCRN data
(2012–2025), run through TurfOps' own algorithms:

| Quantity | Open-Meteo vs. measured | Consequence if it *replaced* the station |
|---|---|---|
| Soil temp 0–7 cm vs. 5 cm probe | annual bias +0.1 °F, r 0.98; daily RMSE 3.1 °F; seasonal bias +1.7 °F (Mar) … −0.9 °F (Jun) | Every crossing **early**: spring 45/50/55 °F = Mar 12 / Apr 5 / Apr 19 vs. Mar 22 / Apr 11 / May 1; fall 70/65/55 = Sep 13 / Sep 25 / Oct 27 vs. Sep 16 / Oct 2 / Oct 31. Mean abs error 4–10 d, worst year 19–51 d, ~half the years within 3 d |
| First fall freeze (Tmin ≤ 32 °F) | median **Nov 13 vs. Oct 26**; 17.5 d late on average, never early, up to 36 d late (modelled nights +2.7 °F warm, +4.1 °F in Nov) | "Last chance to seed" would move from ~Sep 27 to mid-October |
| Soil moisture 0–7 cm vs. 5 cm probe | +0.13 m³/m³ wetter, r 0.65 | The 0.10 drought / 0.40 saturation thresholds do not transfer |
| Humid hours, May–Sep 2025 (hourly) | 6.1 h/day at RH ≥ 90 % vs. 9.0 measured; days with ≥ 10 humid hours 50 vs. 68; nights +2.2 °F warm | Wetness-driven disease risk under-called |
| Air mean, precipitation | +1.2 °F (RMSE 2.2); totals within 1 % | Fine |

**Conclusion: Open-Meteo must never replace the station** for soil, freeze dates, GDD history or
climatology. The only swap where gains outweigh losses is the *forecast*: OWM is also a model
(bias unmeasured), is 3-hourly, stops at 5 days and needs a key.

### What the swap would buy

- **Hourly** points instead of 3-hourly → forecast wet-hour counts stop being quantized (Pythium's
  10 h / 14 h thresholds effectively become 12 h / 15 h on forecast days today).
- **7-day** outlook (up to 16) instead of 5 → the Timing soil outlook sees further.
- **No API key**, exact lat/lon.
- Optional `past_days` → the already-elapsed hours of today/yesterday, which the lake (≈12–24 h
  behind) has not delivered yet. See open decision 2.

### What it would cost

- A new external dependency (Open-Meteo, CC BY 4.0 → attribution required, non-commercial free tier).
- Modelled humidity under-reads (table above) — relevant only if `past_days` is used as "observed".
- Churn in a path that works.

## When to pull the trigger

Implement only if one of these is observed while living with the current build:

1. The Disease Risk banner "current risk reflects the most recent complete day" is a recurring
   annoyance, or today's / forecast bars look wrong or jumpy through the day.
2. Forecast-day disease tiers visibly lag what then gets measured (the 3-hourly quantization).
3. The 5-day horizon is too short for the Timing page's "opening soon" / soil outlook.
4. OpenWeatherMap becomes unreliable, changes its free tier, or the key is a nuisance.

If none of these show up over a season, **close this plan** — the app does not need it.

Not a trigger: the station's failing soil probes (2026). That is a lake-side problem; the fix
order is USCRN 5-minute `subhourly01` (measured) → the app's air→soil regression → Open-Meteo
soil with a per-month bias correction, last. See the Dagster-side spec
`docs/plans/uscrn-soil-dropout-investigation.md`.

## Rate limits (the 429 question)

Free tier: **600 calls/min, 5,000/h, 10,000/day, 300,000/month**. A request for > 10 variables or
> 2 weeks counts as multiple calls (2 weeks × 15 variables = 1.5; 4 weeks = 3.0). The 429 hit
during the evaluation came from three back-to-back 14-year pulls (~365 calls each), nothing
like production use.

TurfOps' usage: **one request, 9 variables, 9 days = 1.0 call**, refreshed on demand only when
the forecast is older than 30 min → **≤ 48 calls/day** (0.5 % of the daily limit).

**The one real risk is self-inflicted.** Today a failed forecast fetch leaves
`last_forecast_refresh` unset, so the *next request retries immediately*. The dashboard polls six
endpoints every 30 s; after one 429 or outage that becomes a retry storm that keeps the limit
tripped. The swap must therefore add a **failure cool-down** in `DataSyncService`:

- on failure record `last_forecast_failure`; do not retry for 5 min, doubling to a 30 min cap;
  reset on success;
- a `Retry-After` header, when present, overrides the computed delay;
- the existing rule "reuse a forecast up to 6 h old" (#217) bridges the gap; past 6 h the app
  degrades to "no forecast" exactly as it does today.

(This cool-down is worth having for OpenWeatherMap too — it is the one piece of this plan that
could be done independently.)

## Open decisions

1. **Remove OpenWeatherMap, or keep it as a fallback?**
   *Recommended: remove.* One source, no key, one code path. Keeping it as a fallback means a
   second client and mixed 1 h / 3 h data that every consumer has to tolerate.
2. **Use `past_days` to fill the same-day gap in Disease Risk?**
   - *Fill the gap, headline = today* (recommended): measured lake hours always win; Open-Meteo's
     past hours top up what the lake has not delivered, so "today" is a full day and the headline
     is never a day behind. Filled days stay flagged as modelled. Caveat: humid hours under-read.
   - *Forecast only*: headline stays on the last fully measured day, as now. Most conservative.
   - *Fill + humidity correction*: fit an RH correction on the 14-year station overlap. Best in
     principle; a calibration to fit, validate and document — larger.

## Backend changes

### 1. New client — `backend/src/datasources/openmeteo.rs` (replaces `openweathermap.rs`)

One keyless GET:

```
https://api.open-meteo.com/v1/forecast
  ?latitude={lat}&longitude={lon}
  &hourly=temperature_2m,apparent_temperature,relative_humidity_2m,precipitation,
          precipitation_probability,weather_code,cloud_cover,wind_speed_10m,wind_gusts_10m
  &temperature_unit=fahrenheit&wind_speed_unit=mph&precipitation_unit=mm
  &timeformat=unixtime&timezone=auto
  &forecast_days=8[&past_days=2]        # past_days only if open decision 2 = fill
```

- 9 variables, ≤ 10 days → 1.0 API call. Do not add variables casually (the 10-variable line).
- Response is column-oriented (`hourly.time[]`, `hourly.temperature_2m[]`, …) plus
  `utc_offset_seconds` — zip into `Vec<ForecastPoint>`, put the offset in
  `ForecastLocation.utc_offset_seconds` (field added in #218). Nulls at the end of the horizon
  → skip the point.
- Same timeouts as today (5 s connect / 15 s request). `test_connection()` = the same call with
  `forecast_days=1`.
- **Do not request soil variables.** Modelled soil would shift the windows by a week (table
  above); the app's regression, trained on the measured probe and anchored to the last
  observation, stays.

### 2. Keep the downstream types; make the step size data-driven

`WeatherForecast` / `ForecastPoint` / `DailyForecast` keep their shape, so the rules, the Timing
outlook and `/soil-temp-forecast` need no change. Two places hard-code 3-hour steps and **must**
change or every forecast wet-hour count triples:

- `logic/disease/weather_days.rs:9` `FORECAST_STEP_HOURS = 3.0` — used for `hours_rh90`,
  `leaf_wetness_hours`, `hours_covered`. Derive the step from consecutive point timestamps
  (or carry `step_hours` on `WeatherForecast`).
- `datasources/openweathermap.rs:278` `hours_covered: 3 * points.len()` — same treatment in the
  new client's daily aggregation. `DailyForecast::is_full_day()` (≥ 18 h) then works unchanged.

`WeatherCondition::from_owm_id` gains a sibling `from_wmo_code` (0 clear · 1–3 clouds · 45/48 fog
· 51–57 drizzle · 61–67, 80–82 rain · 71–77, 85–86 snow · 95–99 thunderstorm).

`WeatherForecast::next_hours(n)` filters by timestamp, so it is step-agnostic; with `past_days`
it must also drop points **before now** (today it only bounds the future side).

### 3. Same-day gap (only if open decision 2 = fill)

`disease::weather_days::build_series` already tops a partly observed day up with forecast points.
With `past_days`, the feed also contains elapsed hours: keep the rule "a fully observed lake day
wins outright", otherwise merge per hour with **lake hours taking precedence**, and keep
`is_forecast = true` on any day containing modelled hours so the UI fades it. The #218 rule
(usable day = ≥ 12 h **and** dawn + afternoon covered) then makes today the headline all day
without special-casing. Rules must keep using only future points.

### 4. Config — `backend/src/config.rs`, `docker-compose.yml`, `.env.example`

- `LAWN_LATITUDE` / `LAWN_LONGITUDE`, **falling back to the existing `OWM_LATITUDE` /
  `OWM_LONGITUDE`** so the deployed `.env` keeps working. Refuse to enable the forecast at 0,0
  (today's silent default when only a key is set).
- `FORECAST_ENABLED` (default true) replaces `OWM_ENABLED` (still honoured as an alias).
- `OWM_API_KEY` becomes unused (or the fallback's switch, per open decision 1).

### 5. Sync service — `backend/src/logic/data_sync.rs`

Swap the client, add the failure cool-down described under "Rate limits", keep the 30 min
staleness and the 6 h reuse rule. `ConnectionStatus.openweathermap` → `forecast` (coordinate
with the frontend; the lake is still reported under the legacy key `soildata`).

### 6. Labels

`DataSource::OpenWeatherMap` → `OpenMeteo` ("Open-Meteo"); check whether the variant is persisted
anywhere (application weather snapshots) before renaming — if it is, keep parsing the old string.

## Frontend changes

- `types/index.ts`: `ConnectionStatus.forecast`; data-source label.
- `pages/Dashboard.tsx`: connection dot "Forecast (Open-Meteo)".
- `pages/DiseaseRisk.tsx` (6 mentions) and the methodology text: "OpenWeatherMap 3-hourly
  forecast" → "Open-Meteo hourly forecast"; if decision 2 = fill, reword the data note about the
  most recent complete day.
- **Attribution (licence requirement, CC BY 4.0):** "Weather forecast data by Open-Meteo.com",
  linked, in the layout footer or Settings, and in the README.

## Tests

- Parse a recorded Open-Meteo response (checked-in fixture): units, offset, null tail.
- Local-day bucketing with hourly points across a UTC-date boundary (extend the #218 test).
- Step arithmetic: 12 hourly humid points = 12 wet hours, not 36; `hours_covered` = 24 for a full day.
- `from_wmo_code` table.
- Cool-down: a failure suppresses the next fetch; success resets it; `Retry-After` honoured.
- If decision 2 = fill: lake hours beat modelled hours; a filled today is usable and flagged
  modelled; rules ignore past points.

## Docs

`CLAUDE.md` (data sources, env vars, key patterns), `README.md` (16 OWM mentions, env table,
attribution), `docs/timing-windows.md` (2), `backend/.env.example` (owner edits — Claude's tools
are blocked from `.env*`).

## Verification before merging

- `cargo fmt --check`, `clippy --all-targets`, `cargo test`, `tsc`, `biome`, production build.
- Run against the live API once: confirm 1 call per refresh in the logs over an hour of normal
  dashboard use, and that a forced failure does not produce a retry storm.
- Compare a day of Disease Risk output before/after on forecast days — tiers should move only
  where 3-hourly quantization was biting.

## Explicitly out of scope

- NWS alerts (frost / freeze / heat) and NWS station observations for a *measured* same-day
  fill — a separate feature; arguably a better answer to the same-day gap than modelled hours.
- Any use of Open-Meteo soil, freeze or history data (see the table — disqualified).
- NCEI normals, PHZ, geocoders, air quality, SPC outlooks (redundant or new features).
