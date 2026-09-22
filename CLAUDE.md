# TurfOps - Lawn Care Management Web Application

## Overview

Containerized web application for tracking lawn care activities with data-driven agronomic recommendations. Rust/Axum backend serves a React SPA frontend. Integrates with a NOAA USCRN weather data lake (Dagster bronze/silver/gold parquet on a mounted filesystem, read via embedded DuckDB), Home Assistant (local patio sensors), and OpenWeatherMap (forecast). Deployed via Docker Compose.

## Tech Stack

- Backend: Rust + Axum + sqlx (PostgreSQL)
- Frontend: React 19 + TypeScript + Vite
- Database: PostgreSQL 18 (app data)
- External Data: NOAA USCRN weather data lake (embedded DuckDB over parquet), Home Assistant API, OpenWeatherMap API
- Deployment: Docker Compose (app + PostgreSQL); the distroless runtime ships `/usr/share/zoneinfo` so `TZ` works
- Async Runtime: Tokio

## Commands

### Backend
- `cd backend && cargo build` — Build backend
- `cd backend && cargo test` — Run tests (263 tests)
- `cd backend && cargo fmt` — Format code
- `cd backend && cargo clippy` — Run linter
- `cd backend && cargo run` — Run API server (needs PostgreSQL)

### Frontend
- `cd frontend && npm install` — Install dependencies
- `cd frontend && npm run dev` — Dev server with API proxy (port 5173)
- `cd frontend && npm run build` — Production build to dist/ (if `npm` is wrapped by Socket and rolldown fails with "loading addons is disabled", run `tsc -b && node node_modules/vite/bin/vite.js build`)
- `cd frontend && npx tsc --noEmit` — Type check
- `cd frontend && npm run lint` — Biome lint + format check
- `cd frontend && npm run format` — Biome auto-format

### Docker
- `docker compose up -d` — Start full stack (port 3000)
- `docker compose down` — Stop all services
- `docker compose build` — Rebuild containers

## Architecture

```
turfops/
├── backend/
│   └── src/
│       ├── main.rs              # Axum server, static file serving
│       ├── config.rs            # Env-var-based configuration
│       ├── error.rs             # Error types with HTTP responses
│       ├── state.rs             # AppState (pool, sync, rules engine, 1 h climate-record memo)
│       ├── api/                 # Route handlers (27 paths / 39 routes) + recommendation_feed.rs shared by dashboard and feed
│       ├── db/                  # PostgreSQL pool, queries, migrations
│       ├── models/              # Data structures (shared with rules); product/ = inventory enums + LLM profile + user-owned facts
│       ├── logic/               # Data sync + 10 agronomic rules + GDD accumulation + seasonal plan + disease risk models + seeding/pre-emergent timing windows
│       └── datasources/         # WeatherLake (DuckDB/parquet), HomeAssistant, OpenWeatherMap, OpenRouter (openrouter/plant.rs + product.rs = prompts + strict JSON schemas)
├── frontend/
│   └── src/
│       ├── App.tsx              # React Router, 14 routes + wildcard
│       ├── api/client.ts        # Fetch wrapper for all API endpoints
│       ├── types/               # TypeScript interfaces matching Rust models (index.ts; disease.ts for disease risk; timing.ts for timing windows; inventory.ts + inventoryTargets.ts for products)
│       ├── pages/               # Dashboard, Applications, Inventory, Landscape, Calendar, Environmental, Recommendations, DiseaseRisk, Timing, SoilTests, SeasonalPlan, Settings
│       └── components/          # Layout, Gauge, AlertCard, TrendChart, GddWidget, NitrogenBudgetWidget, SoilTempForecastWidget, PredictionChart, disease/ (TierBadge, RiskMeter, DailyRiskChart, DiseaseDetail, ManagementPanel, DiseaseRiskWidget), timing/ (StateBadge, WindowTimeline, WindowCard, SoilSeasonChart, SoilSeasonTable, TimingMethod, TimingWidget), inventory/ (StockBadge, AddProductForm, EditProductForm, CategoryFields, ProductRow, ProductDetails)
├── Dockerfile                   # Multi-stage: Node → Rust → slim runtime
└── docker-compose.yml           # app + PostgreSQL 18
```

## API Endpoints

| Method | Path | Purpose |
|--------|------|---------|
| GET | /api/v1/health | Connection status (the lake is reported under the legacy key `soildata`) |
| GET | /api/v1/dashboard | Composite dashboard data |
| GET/PUT | /api/v1/profile | Lawn profile CRUD |
| GET/POST | /api/v1/applications | List (`?type=&limit=&offset=`, type filtered in SQL) / create applications (409 on a true duplicate) |
| PUT/DELETE | /api/v1/applications/:id | Update / delete application |
| GET | /api/v1/applications/calendar | Calendar view |
| GET | /api/v1/environmental | Environmental data (demand-driven refresh) |
| POST | /api/v1/environmental/refresh | Force data refresh |
| GET | /api/v1/recommendations | Active recommendations (`?include_inactive=true` adds answered ones) |
| PATCH | /api/v1/recommendations/:id | Mark addressed/dismissed (with `severity`), or clear both to restore |
| GET | /api/v1/timing-windows | Seeding + spring/fall pre-emergent windows: typical dates, this season's status, freeze dates, soil chart series |
| GET | /api/v1/gdd | GDD accumulation + crabgrass germination model |
| GET | /api/v1/historical | Time-series environmental data (7d/30d/90d) |
| GET | /api/v1/nitrogen-budget | Annual nitrogen budget vs grass-type target |
| GET | /api/v1/seasonal-plan | Seasonal plan with predicted activity windows |
| GET | /api/v1/soil-temp-forecast | 10 cm soil outlook (air→soil regression) + direction-aware threshold crossings |
| GET/POST | /api/v1/soil-tests | List / record soil tests |
| PUT/DELETE | /api/v1/soil-tests/:id | Update / delete a soil test |
| GET | /api/v1/soil-tests/recommendations | pH / N-P-K / micronutrient advice from the latest test |
| GET/POST | /api/v1/plants | List / add landscape plants (add generates a care plan via OpenRouter) |
| GET/PUT/DELETE | /api/v1/plants/:id | Read / update / delete a plant |
| POST | /api/v1/plants/:id/refresh-plan | Regenerate a plant's care plan |
| GET/POST | /api/v1/products | List (`?category=&include_archived=`) / add a shelf product (LLM profile when configured and `assist` ≠ false; saves by hand otherwise — `category` then required) |
| GET/PUT/DELETE | /api/v1/products/:id | Read / replace user-owned fields (never the profile) / delete |
| POST | /api/v1/products/:id/refresh-profile | Regenerate the LLM profile; returns `suggested_facts` + `changed_fields`, does not overwrite the user's facts |
| GET | /api/v1/frac-classes | FRAC class options for the application form + the classes a `?product=` name resolves to |
| GET | /api/v1/disease-risk | Per-disease risk tiers, 11-day series, contributing factors, methodology |

## Data Sources

- **Ambient (temp/humidity)**: Home Assistant API → patio sensor
- **Soil (temp/moisture)**: Weather data lake silver hourly parquet → NOAA USCRN PA Avondale (WBANNO 3761). The live reading (`datasources/weather/latest.rs`) is each sensor's newest non-null value within 24 h, not the newest row — the probes drop individual (mostly daytime) hours, so the newest row is often there with empty soil columns
- **Precipitation**: Weather data lake silver hourly parquet → NOAA measured values
- **Forecast**: OpenWeatherMap API (5-day / 3-hourly). Daily summaries are bucketed by the **location's local date** (`city.timezone`) and carry `hours_covered`; anything that needs a daily mean or judges a whole day (`WeatherForecast::full_days()`: soil outlooks, application window, hot-day streak) skips the partial first/last day
- Lake layers: **silver** (`silver_weather.parquet`, hourly cleaned/deduped, °C/mm/% native units) backs the live reading, 7-day summary, trend, and `/historical`; **gold** (`daily_weather.parquet`, daily means already in °F + precomputed `gdd50`) backs the seasonal plan, soil-temp regression, and GDD. The timing windows read both: daily **5 cm** soil means aggregated from silver (gold only has 10 cm) plus gold's daily air min/mean + `gdd50` (`datasources/weather/climatology.rs`). Read in-process via embedded DuckDB (`datasources/weather.rs`).

## Key Patterns

- Demand-driven data refresh: sensors stale after 5min, forecast after 30min. Zero external calls when idle. A failed lake read keeps the last good soil summary (it used to wipe it for 5 min); a newest lake reading older than 72 h is **not** used as current (`soil_observed_at` carries the real observation time and the dashboard shows a banner); a forecast up to 6 h old survives a failed OWM fetch. (Lake parquet reads are local + fast, so soil/weather is re-read on each refresh rather than cached in Postgres.)
- GDD (Growing Degree Days, base 50°F): the gold layer precomputes daily `gdd50` (verified identical to the app's own `((max+min)/2 - 50).max(0)` formula); the app sums it to a YTD running total on demand (`gdd::accumulate_daily_gdd`) and passes it to rules via `EnvironmentalSummary.gdd_base50_ytd`. No `gdd_daily` cache table.
- Disease risk (`logic/disease/`): one pure model per disease over a daily weather series — brown patch (Fidanza E-index), dollar spot (Smith-Kerns), Pythium blight (Nutter-criteria score), gray leaf spot + red thread (experimental suitability indices, `validated: false`). Each keeps its native score; only the Low/Moderate/High/Severe tier is comparable, so there is no blended score. Observed days come from silver hourly aggregated per local day in DuckDB (`datasources/weather/disease.rs`); the lake lags ~1 day, so today's remainder + the outlook come from the OWM 3-hourly forecast (`weather_days::build_series`). The headline falls back to the last usable day: a day needs ≥12 h of data **and** both its pre-dawn (03–08) and afternoon (13–17) hours — 12 h of forecast afternoon/evening has no dawn low and none of the humid night. Overseeding (≤60 d) amplifies gray leaf spot; no fertilizer in 60 d amplifies red thread — only the headline tier is raised (`tier_note` explains it); the daily series stays weather-only.
- Disease management (`logic/disease/management/`): tier → action (Low none · Moderate monitor · High/Severe apply preventative, or `Protected` when a logged fungicide that is ≥ Good on that disease is within its residual window: 21 d systemic / 14 d contact, −7 d under Severe). Each disease has cultural practices plus preventative and curative programs from a static FRAC efficacy matrix (`programs.rs`); the suggested class is the best non-restricted option that isn't the class used last. The matrix was checked against Univ. of Kentucky PPA-1 2024 (1–4 ratings: 4/3.5 → Excellent, 3/2.5 → Good, 2/1.5 → Fair; each option's source rating is in a comment) — re-check against the current edition before changing a rating. `protects_at_severe: false` marks chemistry that counts as protection at High but not Severe (strobilurins on Pythium). A forecast High/Severe day within 3 days gets a heads-up at Low and Moderate alike, and "covers through…" is judged **on that day at that tier** (`headline::Outlook`), not from today's protection status. A logged fungicide's classes are `frac_class::classes_of(app)`: the `frac_classes` recorded on the form (nullable `TEXT[]` column; the picker pre-fills from the product name and warns when the product is unrecognized), else every class the product name resolves to (`PRODUCT_PATTERNS`, premixes → several). A premix yields one `FungicideRecord` per class, so it protects and counts toward rotation for each; an unknown product with no class earns neither. Ratings are per FRAC class even though actives within a class differ; a split class is omitted from that disease rather than averaged. Multi-site codes: mancozeb = M3, chlorothalonil = M5. No application rates are ever given — label governs. There is deliberately no "I see symptoms" toggle: the curative program is always shown for the user to apply on their own judgment. Red thread's remedy is nitrogen, never a spray.
- Disease recommendations: the five old heuristic disease rules were replaced by `disease::recommendations::to_recommendations` (High/Severe only; `Protected` → Info, red thread → Advisory), appended in the dashboard and recommendations handlers so the feed always agrees with the Disease Risk page. A lake/forecast outage degrades to no disease alerts.
- Timing windows (`logic/timing/`, full method in `docs/timing-windows.md`): fall/spring/dormant seeding + spring/fall pre-emergent. A window is boundaries (`windows.rs`: `Soil` crossing · `BeforeFirstFreeze(days)` · `Gdd` · `Earliest([..])`) on the 5-day mean **5 cm** soil temp; a crossing must hold 5 days (`series::detect`, state-based, gaps reset the run). Each boundary is located per historical year (≤15 complete years with ≥300 soil days) → median/p10/p90/earliest/latest (`climatology::date_stat`, offsets from Jan 1 of the season year so dormant seeding survives the new year), then resolved for this season as Observed / Tentative (<5 days held) / Forecast (air→soil regression refit on 5 cm; never dated before today) / Typical. A Tentative crossing of a **closing** boundary makes the window `Closing`, not `Closed` — it must hold 5 days before spring pre-em is called missed. With stale data (>5 d) only a confirmed crossing counts; a run in progress or a forecast-only one falls back to the calendar. A typical date that has passed while station data is fresh **and has reported past it** is **overdue and gets no date** — never invent one (inside the lake's lag it stays the estimate; "running behind" wording only past p90); stale data (>5 d) or an ended scan range falls back to the calendar. Historical crossings flagged `after_gap` (run starts on the first day after a data hole) are dropped — they date a sensor outage. The soil outlook joins station air + forecast air into one series, interpolating the hole between them (≤7 d) so lagged drivers exist, and anchors to the last observed soil value. `today` is station-local (`ClimateRecord::local_today`, from the lake's UTC offset), not the server clock. Freeze dates come from gold `air_temp_min_f` ≤ 32 and count only when the record on the side that matters is ≥90 % complete (before a first fall freeze from Sep 15, after a last spring freeze to May 31); freeze-anchored boundaries always use the *typical* first freeze (a forecast freeze is already too late to seed). Seed and pre-emergent never share a season (`log.rs`): logged `Overseed` blocks that season's pre-emergent and vice versa — user rule, no toggle. No rates — label governs. The 15-year lake read is memoized 1 h in `AppState::climate_cache`; nothing is cached in Postgres so the current season is always live. Deliberate departures from lawn-answers.com (per-year crossings not one averaged curve; seeding never closes on a forecast freeze; spring pre-em closes at 55°F/200 GDD not 50°F) are tabled in the doc.
- Timing recommendations + plan: the old 10 cm `pre_emergent` and `fall_overseeding` rules were replaced by `timing::recommendations::to_recommendations` (Primary windows only; OpeningSoon → Info, Open/Ideal → Advisory, Closing → Warning, spring pre-em Closed ≤21 d → Warning; fall pre-em capped at Info/Advisory since it is the alternative to seeding), appended in the dashboard and recommendations handlers next to disease risk. Ids are `timing_<window>_<season_year>` (e.g. `timing_fall_seeding_2026`). `timing::plan::activities` builds the seasonal plan's `pre_emergent`, `fall_pre_emergent`, `fall_overseeding` and `core_aeration` (same window as seeding) from the windows' typical dates, with this season's **status** taken from the window's resolved state so the plan never contradicts the Timing page; `api/seasonal_plan.rs` removes the legacy 10 cm version of everything in `plan::owned_ids(grass)` and adds the timing activities, so a lake outage or a warm-season lawn keeps the legacy activity. A window blocked by the log is omitted from the plan — and its legacy twin stays removed. A lake outage degrades the feed to no timing recommendations.
- Timing UI (`/timing`): window-state colors are the status palette + neutrals in `types/timing.ts`, always with symbol + label (`StateBadge`). `WindowTimeline` is one blue ramp (spread → typical → ideal). `SoilSeasonChart` uses a CVD-validated categorical pair (this year `#2a78d6`, forecast = same hue dashed; last year `#eb6834`) with the typical year as a neutral dotted reference, 10°F ticks, the current season's thresholds (fall set from July), and a table view. A boundary with `date: null` renders as "Later than usual — not yet seen", never a date. The dashboard `TimingWidget` lists Primary windows sorted by `byUrgency`.
- Disease Risk UI (`/disease-risk`, `/disease-risk/:slug`): tier colors are the status palette in `types/disease.ts` and are never used alone — always symbol + label, with text in ink colors. Forecast bars are faded, today is outlined, and every chart has a table view.
- 10 agronomic rules are pure functions — no IO, no UI dependencies. 3 rules (grub control, spring nitrogen, broadleaf herbicide) use GDD for enhanced timing/urgency. Pre-emergent, fall overseeding **and core aeration** are not rules: anything that must agree with the seeding window comes from the timing windows (`logic/timing/companions.rs`: aeration = fall seeding window Open/Ideal/Closing, silent once aerated/seeded or when a fall pre-em is logged; fall `broadleaf_fall` is dropped if seeded, untouched if fall pre-em logged or seeding closed, else Advisory with an "only if NOT seeding" caveat). There is no 10 cm pre-emergent/crabgrass/seeding advice anywhere — the old `SoilTempForecastRule` (which never fired) is deleted and the `/soil-temp-forecast` thresholds are direction-aware (45 falling, 60 rising, 65 both, 75 rising).
- `rules/nitrogen_guard.rs` runs after the rules: "apply N" recs (`fall_fert_*`, `spring_n_ready`) go **on hold** (Info) while `fertilizer_block` or a Warning+ `heat_stress_forecast` is active, carry the remaining annual N budget, are capped to it, and become "target reached" at zero. Lawn nitrogen is always `Application::turf_nitrogen_lbs()` (plant applications excluded).
- Rules gracefully degrade when GDD data is `None` — all GDD-enhanced logic is additive
- Recommendation state (addressed/dismissed) is persisted in Postgres (`recommendation_states`, keyed by recommendation id) together with the severity that was answered. An answer is **not permanent** (`models/recommendation_state.rs`, the one place the policy lives): the alert returns when it escalates past the answered severity (Info "opening soon" → Warning "closing"; Protected Info → Severe) or when its episode ends — 7 d for weather alerts (irrigation, heat, rain delay, mowing, `fertilizer_block`), 21 d for disease (fungicide residual), 120 d for seasonal work; `application_followup_<id>` never expires. Rules therefore don't need a year in their id. `GET /recommendations?include_inactive=true` also returns the answered ones (flagged) for the page's "Show dismissed / addressed" list; PATCH with both flags false deletes the row (Restore). Dashboard alerts and the Recommendations page share `api/recommendation_feed.rs`. Disease recommendation ids are `disease_<slug>` (e.g. `disease_brown_patch`)
- All temperatures stored in Fahrenheit (convert from Celsius at ingestion)
- Axum serves React SPA static files with fallback to index.html for client-side routing
- Seasonal plan uses historical NOAA soil temp data (up to 10 years) to predict activity windows via threshold crossing analysis; crossings cached in DB per station (`seasonal_threshold_crossings.station_wbanno`); a year is only "settled" once recomputed after it ended, so the year in progress is recomputed on each load instead of freezing at its first crossing
- Calendar view overlays seasonal plan activity windows (status-colored bars) alongside application dots; detail panel shows both when a date is selected
- `GrassType::Mixed` is a **cool-season** blend (fescue / bluegrass / rye) — it gets the rules, seeding windows and disease models. Warm-season lawns get no cool-season plan activities (`seasonal_plan::COOL_SEASON_ONLY`: spring N, fall feedings, winterizer, fall seeding + aeration); weed and grub control stay.
- Product inventory (`models/product/`, `api/products.rs`, `/inventory`; plan in `docs/plans/product-inventory.md`): what is on the shelf, by `ProductCategory` (Fertilizer · Supplement · Fungicide · Herbicide · InsectControl · Seed · SoilAmendment · Surfactant · Other). **No quantities by design** — `stock_status` is a hand-set In stock / Low / Out (Out = rebuy; `archived` = no longer used). A product has user-owned **facts** (category, form, NPK, FRAC classes, herbicide timing, `ProductTarget` list, amendment kind, optional label rate) flattened into the JSON, plus an optional LLM **profile** (JSONB, `profile_model` / `profile_generated_at`) that is informational and regenerable. On add, the assistant fills the facts once (curated `frac_classes_for_product` beats the LLM's FRAC list; the user's category/form beat its guess); PUT edits facts only; refresh stores the new profile and returns a diff for the user to apply. The LLM schema has no interval or timing fields; its one rate is `suggested_label_rate`, stored as the product's label rate, shown with a "verify against your label" caveat, and used only to pre-fill the application form — never in advice text. A product saves without OpenRouter (unlike plants). Every enum's variant name is its serde name, DB string and JSON-schema value (`models/product/enums.rs` `string_enum!`).
- Landscape-plant tasks use the window in force *today* (`plant_maintenance::window_on`), so one that wraps the new year (Dec → Feb) stays open in January with the same recommendation id; completion is scoped to that window (from 30 d before it opens), so April's feeding does not complete September's.
- Mowing is tracked as an ApplicationType (no cut height field); shows on calendar and applications list like any other type

## Environment Variables

See `backend/.env.example` for full list:
- `DATABASE_HOST`, `DATABASE_PORT`, `DATABASE_NAME`, `DATABASE_USER`, `DATABASE_PASSWORD` — App PostgreSQL connection
- `DATALAKE_ROOT` — Mount point of the NOAA weather data lake (default `/data`); silver/gold weather parquet paths derive beneath it. Override individually with `WEATHER_SILVER_PATH` / `WEATHER_GOLD_PATH`.
- `NOAA_STATION_WBANNO` — USCRN station filter (default 3761)
- `HA_URL`, `HA_TOKEN` — Home Assistant connection
- `OWM_API_KEY` — OpenWeatherMap API key (an empty value = not configured, same for the other optional keys)
- `OPENROUTER_API_KEY`, `OPENROUTER_MODEL`, `OPENROUTER_ENABLED` — landscape-plant care plans (503 without a key)
- `CORS_ALLOWED_ORIGIN` — only when the SPA is served from another origin
- `TZ` — the lawn's timezone for every `Local::now()` date gate (compose default `America/New_York`)
- `DB_PASSWORD` — required by compose (no default); flows to Postgres and `DATABASE_PASSWORD`
- `LAWN_*` — Default lawn profile settings

## Agronomic Thresholds (TTTF Zone 7a)

| Metric | Threshold | Meaning |
|--------|-----------|---------|
| Soil temp 10cm | 60-75°F | Grub control window |
| Ambient temp | >85°F | Fertilizer stress risk |
| Soil moisture | <0.10 | Irrigation needed |
| Soil moisture | >0.40 | Saturated - avoid fertilizer |
| Humidity | >80% | Reference line on the Environmental humidity chart (disease risk now comes from the per-disease models) |
| Brown patch E-index | ≥5 / ≥6 | High / Severe |
| Dollar spot probability (Smith-Kerns) | ≥20% / ≥40% | High (published action threshold) / Severe |
| Pythium score (0–5) | ≥3 / ≥4 | High / Severe |
| Soil temp 5cm (5-day mean, held 5 d) | 45 / 50 / 55°F rising | Spring pre-emergent: early / ideal / closed (or 200 GDD) |
| Soil temp 5cm (5-day mean, held 5 d) | 70 / 65 / 55°F falling | Fall pre-emergent: opens / late / closed |
| Soil temp 5cm (5-day mean, held 5 d) | ≤75°F from Aug 15 → 55°F | Fall seeding opens → soil-side close |
| First fall freeze (median, air min ≤32°F) | −45 d / −30 d (KBG 60/45, PRG 35/21) | Fall seeding: end of ideal / last chance |
| GDD (base 50°F) | 500-700 | Grub control (egg-laying → peak hatch) |
| GDD (base 50°F) | 50-150 | Spring nitrogen readiness / broadleaf herbicide spring window |
