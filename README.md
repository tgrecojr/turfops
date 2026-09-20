# TurfOps

A containerized web application for tracking lawn care activities and providing data-driven agronomic recommendations. Built with a Rust/Axum backend serving a React SPA frontend, deployed via Docker Compose.

## Features

- **Application Tracking**: Log fertilizer, pre-emergent, fungicide, mowing, and other lawn treatments
- **Environmental Data**: Real-time soil temperature, moisture, and ambient conditions from multiple sources
- **Smart Recommendations**: 14 agronomic rules provide data-driven alerts for optimal treatment timing
- **Disease Risk**: Per-disease risk from published models (brown patch, dollar spot, Pythium blight, gray leaf spot, red thread) — each with a Low/Moderate/High/Severe tier, an 11-day trend and outlook, contributing factors, a "how this is calculated" breakdown, and preventative/curative guidance that knows what you've already sprayed
- **Calendar View**: Visualize application history and seasonal plan activity windows with colored indicators
- **Seasonal Plan Integration**: Calendar overlays predicted activity windows from the seasonal plan alongside actual applications
- **Landscape Maintenance** *(optional)*: Track shrubs, trees, and perennials alongside turf. Enter a plant by common or scientific name and get a homeowner-level care plan (pruning windows, fertilizing, mulching) that overlays Calendar, Seasonal Plan, and Recommendations. Powered by an LLM through OpenRouter and cached per plant.
- **FRAC Rotation**: Fungicide resistance management — the suggested class for each disease avoids the one you used last, and a logged application counts as protection for its residual window
- **Demand-Driven Refresh**: Sensor data refreshes only when viewed (5-min staleness for sensors, 30-min for forecasts)

## Architecture

```
┌──────────────────────────────────────────────────────┐
│  Docker Compose                                       │
│                                                       │
│  ┌────────────────────────────────────────────────┐  │
│  │  turfops-app (port 3000)                        │  │
│  │  ┌─────────────┐   ┌────────────────────────┐  │  │
│  │  │ React SPA   │   │ Axum API Server        │  │  │
│  │  │ (static)    │◄──│  /api/v1/* endpoints   │  │  │
│  │  └─────────────┘   │  14 rules + disease    │  │  │
│  │                     │  3 datasource clients  │  │  │
│  │                     └───────────┬────────────┘  │  │
│  └─────────────────────────────────┼───────────────┘  │
│                                    │                   │
│  ┌─────────────────┐               │                   │
│  │ PostgreSQL 16   │◄──────────────┘                   │
│  │ (app data)      │                                   │
│  └─────────────────┘                                   │
└───────────────────────────────────────────────────────┘
        │                    │                │
  Weather data lake    Home Assistant    OpenWeatherMap
  (NOAA USCRN parquet) (patio sensors)    (forecast)
```

## Tech Stack

- **Backend**: Rust + Axum + sqlx (PostgreSQL)
- **Frontend**: React 19 + TypeScript + Vite
- **Database**: PostgreSQL 16 (app data)
- **Weather data**: NOAA USCRN data lake (parquet) read in-process with embedded DuckDB
- **Deployment**: Docker Compose

## Docker Image

Pre-built Docker images are published to GitHub Container Registry on every push to `main`.

```bash
docker pull ghcr.io/tgrecojr/turfops:latest
```

Tagged releases are also available by version (e.g., `ghcr.io/tgrecojr/turfops:1.0.0`).

## Quick Start

### Prerequisites

- Docker and Docker Compose
- (Optional) NOAA USCRN weather data lake (silver/gold parquet) mounted read-only — soil data, GDD, seasonal plan, and disease risk all depend on it
- (Optional) Home Assistant instance with temperature/humidity sensors
- (Optional) OpenWeatherMap API key for forecast-based rules and the disease-risk outlook
- (Optional) [OpenRouter](https://openrouter.ai) API key to enable the Landscape Maintenance feature

### 1. Configure

You can either use the pre-built image from GHCR or build from source.

**Option A: Use the pre-built image (recommended)**

```bash
# Download docker-compose.yml and .env.example
curl -O https://raw.githubusercontent.com/tgrecojr/turfops/main/docker-compose.yml
curl -O https://raw.githubusercontent.com/tgrecojr/turfops/main/.env.example
cp .env.example .env
```

Update `docker-compose.yml` to use the pre-built image instead of building locally — replace the `build:` block under the `app` service with:

```yaml
image: ghcr.io/tgrecojr/turfops:latest
```

**Option B: Build from source**

```bash
git clone git@github.com:tgrecojr/turfops.git
cd turfops
cp .env.example .env
```

Edit `.env` with your values (see [Environment Variables](#environment-variables) below).

### 2. Start the Application

```bash
docker compose up -d
```

This starts two containers:
- **app** — TurfOps web application on port 3000
- **db** — PostgreSQL 16 database with persistent volume

### 3. Open in Browser

Navigate to `http://localhost:3000`. The dashboard will load with live environmental data from your configured sources.

### 4. Stop

```bash
docker compose down
```

Data persists in the `turfops_data` Docker volume. To fully reset, add `-v`:

```bash
docker compose down -v
```

## Environment Variables

All configuration is done through environment variables. When running with Docker Compose, set these in a `.env` file in the project root.

### Database

| Variable | Description | Default |
|----------|-------------|---------|
| `DATABASE_HOST` | App PostgreSQL hostname | `localhost` |
| `DATABASE_PORT` | App PostgreSQL port | `5432` |
| `DATABASE_NAME` | App database name | `turfops` |
| `DATABASE_USER` | App database user | `turfops` |
| `DATABASE_PASSWORD` | App database password | **required** |
| `DB_PASSWORD` | Password used by the PostgreSQL container (Docker Compose) | `turfops_dev` |
| `DB_MAX_CONNECTIONS` | Maximum database connection pool size | `10` |

> **Note**: When using Docker Compose, `DATABASE_HOST`, `DATABASE_PORT`, `DATABASE_NAME`, and `DATABASE_USER` are pre-configured in `docker-compose.yml`. You only need to set `DB_PASSWORD` in your `.env` to change the database password (it flows to both the PostgreSQL container and the app's `DATABASE_PASSWORD`).

### Lawn Profile

These set the default lawn profile created on first startup. They can be changed later via the Settings page.

| Variable | Description | Default |
|----------|-------------|---------|
| `LAWN_NAME` | Display name for your lawn | `Main Lawn` |
| `LAWN_GRASS_TYPE` | Grass species | `TallFescue` |
| `LAWN_USDA_ZONE` | USDA hardiness zone | `7a` |
| `LAWN_SOIL_TYPE` | Soil type | `Loam` |
| `LAWN_SIZE_SQFT` | Lawn area in square feet | `5000` |
| `LAWN_IRRIGATION_TYPE` | Irrigation system type | `InGround` |

Valid grass types: `KentuckyBluegrass`, `TallFescue`, `PerennialRyegrass`, `FineFescue`, `Bermuda`, `Zoysia`, `StAugustine`

Valid soil types: `Clay`, `ClayLoam`, `Loam`, `SandyLoam`, `Sand`

Valid irrigation types: `InGround`, `Hose`, `Manual`, `None`

### Weather Data Lake (NOAA USCRN)

NOAA USCRN observations are read from a Dagster-built data lake: parquet files on a filesystem mounted read-only into the container and queried in-process with embedded DuckDB. There is no database connection to configure.

| Variable | Description | Default |
|----------|-------------|---------|
| `DATALAKE_HOST_PATH` | Host path of the lake, bind-mounted to `/data` (read-only) by Docker Compose | `/data` |
| `DATALAKE_ROOT` | Lake mount point inside the container; the silver/gold paths derive beneath it | `/data` |
| `WEATHER_SILVER_PATH` | Override for the hourly silver parquet | `$DATALAKE_ROOT/silver/weather/silver_weather.parquet` |
| `WEATHER_GOLD_PATH` | Override for the daily gold parquet | `$DATALAKE_ROOT/gold/weather/daily_weather.parquet` |
| `NOAA_STATION_WBANNO` | NOAA USCRN station ID | `3761` (PA Avondale) |

- **Silver** (hourly, °C / mm / % RH) backs the live soil reading, 7-day summary, trend charts, and the **disease risk models** (air temp, relative humidity, precipitation).
- **Gold** (daily, °F, precomputed `gdd50`) backs GDD, the seasonal plan, and the soil-temperature forecast.

> **Note**: The container runs as uid 65532 and needs read access to the mounted files. The lake is loaded about once a day, so station data trails real time by up to a day — see [Disease Risk](#disease-risk).

### Home Assistant

Connect to a Home Assistant instance to read local temperature and humidity sensors (e.g., a patio sensor).

| Variable | Description | Default |
|----------|-------------|---------|
| `HA_URL` | Home Assistant base URL | *(empty — disabled if not set)* |
| `HA_TOKEN` | Long-lived access token | *(empty)* |
| `HA_TEMPERATURE_ENTITY` | Entity ID for temperature sensor | `sensor.temp_humidity_sensor_temperature` |
| `HA_HUMIDITY_ENTITY` | Entity ID for humidity sensor | `sensor.temp_humidity_sensor_humidity` |
| `HA_TEMPERATURE_UNIT` | Unit reported by sensor (`fahrenheit` or `celsius`) | `fahrenheit` |

To generate a long-lived access token: Home Assistant → Profile → Long-Lived Access Tokens → Create Token.

### OpenWeatherMap (Optional)

Enables forecast-based rules (rain delay, heat stress warnings, optimal application windows) and supplies today's remaining hours plus the 4-day outlook for disease risk.

| Variable | Description | Default |
|----------|-------------|---------|
| `OWM_API_KEY` | OpenWeatherMap API key | *(empty — disabled if not set)* |
| `OWM_LATITUDE` | Location latitude | *(none)* |
| `OWM_LONGITUDE` | Location longitude | *(none)* |
| `OWM_ENABLED` | Enable/disable OWM integration | `true` |

Sign up for a free API key at [openweathermap.org](https://openweathermap.org/api). The free tier (1,000 calls/day) is more than sufficient.

### OpenRouter (Optional — Landscape Maintenance)

Enables the **Landscape** page, which generates a homeowner-level maintenance plan for each plant you add (pruning windows, fertilizing, mulching, deadheading, winter protection). Plans are generated once per plant through an LLM on [OpenRouter](https://openrouter.ai) and cached in Postgres, so there is no recurring per-view cost — only on plant creation or a manual "Regenerate plan" click.

| Variable | Description | Default |
|----------|-------------|---------|
| `OPENROUTER_API_KEY` | OpenRouter API key | *(empty — feature disabled if not set)* |
| `OPENROUTER_MODEL` | Model identifier | `anthropic/claude-haiku-4-5` |
| `OPENROUTER_ENABLED` | Enable/disable the feature | `true` |
| `OPENROUTER_BASE_URL` | API base URL | `https://openrouter.ai/api/v1` |

> **Feature flag behavior**: The **presence of `OPENROUTER_API_KEY` is the feature flag** — there is no separate toggle. `OPENROUTER_ENABLED` is a second kill-switch you can flip without rotating the key. When disabled:
>
> - The `/landscape` page still loads, but adding a plant or regenerating a plan returns a clear `503 — OpenRouter not configured` error.
> - **Existing cached plans stay visible.** Plant rows you added earlier still appear on the Landscape page, and their maintenance windows still overlay Calendar, Seasonal Plan, and Recommendations. Only *new* plan generation and *regenerating* existing plans are blocked.
> - All turf features continue to work unchanged.

### Server

| Variable | Description | Default |
|----------|-------------|---------|
| `SERVER_HOST` | Bind address | `0.0.0.0` |
| `SERVER_PORT` | HTTP port | `3000` |
| `STATIC_DIR` | Path to frontend static files | `/app/static` (in container) |
| `RUST_LOG` | Log level (`error`, `warn`, `info`, `debug`, `trace`) | `info` |

### Example `.env` File

```env
# Database (DB_PASSWORD flows to both PostgreSQL container and app)
DB_PASSWORD=my_secure_password
# These are pre-configured in docker-compose.yml but can be overridden:
# DATABASE_HOST=db
# DATABASE_PORT=5432
# DATABASE_NAME=turfops
# DATABASE_USER=turfops

# Lawn profile
LAWN_NAME=Front Yard
LAWN_GRASS_TYPE=TallFescue
LAWN_USDA_ZONE=7a
LAWN_SOIL_TYPE=ClayLoam
LAWN_SIZE_SQFT=8000
LAWN_IRRIGATION_TYPE=InGround

# NOAA station
NOAA_STATION_WBANNO=3761

# Weather data lake (host path mounted read-only at /data)
DATALAKE_HOST_PATH=/path/to/datalake
DATALAKE_ROOT=/data

# Home Assistant
HA_URL=http://192.168.1.50:8123
HA_TOKEN=your_long_lived_access_token_here
HA_TEMPERATURE_ENTITY=sensor.temp_humidity_sensor_temperature
HA_HUMIDITY_ENTITY=sensor.temp_humidity_sensor_humidity
HA_TEMPERATURE_UNIT=fahrenheit

# OpenWeatherMap
OWM_API_KEY=your_api_key_here
OWM_LATITUDE=40.71
OWM_LONGITUDE=-74.01

# OpenRouter (optional — enables the Landscape Maintenance feature)
OPENROUTER_API_KEY=your_openrouter_key_here
# OPENROUTER_MODEL=anthropic/claude-haiku-4-5
# OPENROUTER_ENABLED=true

# Logging
RUST_LOG=info
```

## Data Sources

| Source | Data Provided | Connection |
|--------|--------------|------------|
| **Weather data lake** | NOAA USCRN hourly soil temperature (5/10/20/50/100cm), soil moisture, air temperature, relative humidity, precipitation; daily aggregates + GDD | Parquet on a read-only mount via `DATALAKE_ROOT`, read with embedded DuckDB |
| **Home Assistant** | Ambient temperature, humidity (patio sensor) | REST API via `HA_URL` + `HA_TOKEN` |
| **OpenWeatherMap** | 5-day/3-hour forecast (temp, rain, humidity, wind) — also fills in today and the outlook for disease risk | REST API via `OWM_API_KEY` |

## API Endpoints

| Method | Path | Purpose |
|--------|------|---------|
| `GET` | `/api/v1/health` | Connection status for all datasources |
| `GET` | `/api/v1/dashboard` | Composite dashboard (profile, env summary, alerts, recent apps) |
| `GET` | `/api/v1/profile` | Current lawn profile |
| `PUT` | `/api/v1/profile` | Update lawn profile |
| `GET` | `/api/v1/applications?type=X` | List applications (optional type filter) |
| `POST` | `/api/v1/applications` | Create new application |
| `PUT` | `/api/v1/applications/:id` | Update application |
| `DELETE` | `/api/v1/applications/:id` | Delete application |
| `GET` | `/api/v1/applications/calendar?year=Y&month=M` | Applications grouped by date |
| `GET` | `/api/v1/environmental` | Environmental data with demand-driven refresh |
| `POST` | `/api/v1/environmental/refresh` | Force immediate data refresh |
| `GET` | `/api/v1/recommendations` | Active recommendations: rules engine + disease risk (High/Severe) + plant maintenance, follow-ups, soil tests |
| `PATCH` | `/api/v1/recommendations/:id` | Mark recommendation addressed/dismissed |
| `GET` | `/api/v1/disease-risk` | Per-disease risk: tier, score, 11-day series, contributing factors, methodology, management plan |
| `GET` | `/api/v1/gdd?year=Y` | GDD accumulation + crabgrass germination model |
| `GET` | `/api/v1/historical?range=7d\|30d\|90d` | Time-series environmental data for trend charts |
| `GET` | `/api/v1/nitrogen-budget` | Annual nitrogen applied vs. grass-type target |
| `GET` | `/api/v1/soil-temp-forecast` | Predicted soil temperatures and threshold crossings |
| `GET` | `/api/v1/seasonal-plan?year=Y` | Full year of predicted activity windows (turf + plants) |
| `GET` | `/api/v1/soil-tests` | List soil tests |
| `POST` | `/api/v1/soil-tests` | Record a soil test |
| `PUT` | `/api/v1/soil-tests/:id` | Update a soil test |
| `DELETE` | `/api/v1/soil-tests/:id` | Delete a soil test |
| `GET` | `/api/v1/soil-tests/recommendations` | pH and nutrient recommendations from the latest test |
| `GET` | `/api/v1/plants` | List plants for the active profile |
| `POST` | `/api/v1/plants` | Add a plant — backend calls OpenRouter and caches the plan |
| `GET` | `/api/v1/plants/:id` | Single plant with cached maintenance plan |
| `PUT` | `/api/v1/plants/:id` | Update plant metadata (name, location, notes) |
| `DELETE` | `/api/v1/plants/:id` | Delete a plant |
| `POST` | `/api/v1/plants/:id/refresh-plan` | Regenerate the cached plan via OpenRouter |

## Pages

| Page | Description |
|------|-------------|
| **Dashboard** | Gauges for soil temp, ambient temp, humidity, and soil moisture. GDD, disease risk, nitrogen budget, and soil-temp forecast widgets. Active alerts and recent applications. Auto-refreshes every 30 seconds. |
| **Applications** | Filterable table of all lawn treatments including mowing. Add new applications with type, product, rate, and notes. |
| **Calendar** | Month grid view with colored dots for applications and status-colored bars for seasonal plan activity windows. Plant-maintenance windows render as outlined bars (distinct from filled turf bars). Click any date to see details grouped into Applications, Turf Activities, and Plant Maintenance. |
| **Landscape** | Add plants by common or scientific name to get a homeowner-level care plan (pruning, fertilizing, mulching, etc.) per plant. Each card shows the plan summary, task windows, warnings, and a "Regenerate plan" button. **Requires `OPENROUTER_API_KEY`** — see [OpenRouter](#openrouter-optional--landscape-maintenance). |
| **Environmental** | Detailed sensor data, soil depth readings, 7-day trends and averages. |
| **Recommendations** | Active recommendations from the rules engine, including a new **Plant Maintenance** category when a plant's care window is open. Diseases at High or Severe risk appear here too. Mark as addressed or dismiss. |
| **Disease Risk** | One row per disease with its score, tier meter, and Low/Moderate/High/Severe badge. Select a disease for its 11-day chart (observed + forecast), what to do now, cultural practices, preventative and curative fungicide programs, contributing factors, and how the score is calculated. See [Disease Risk](#disease-risk). |
| **Soil Tests** | Record lab results and get pH (lime/sulfur) and nutrient recommendations. |
| **Seasonal Plan** | Full-year timeline of predicted activity windows. When plants are configured, a **All / Turf / Plants** filter appears so you can view them separately. |
| **Settings** | Edit lawn profile (grass type, zone, soil type, size, irrigation). |

## Development

### Prerequisites

- Rust 1.88+
- Node.js 20+
- PostgreSQL 16 (or use `docker compose up db` for just the database)

### Backend

```bash
cd backend

# Set up environment
cp .env.example ../.env
source ../.env  # or use dotenvy

# Run database migrations
export DATABASE_URL=postgres://turfops:turfops_dev@localhost:5433/turfops
cargo run  # migrations run automatically on startup

# Development commands
cargo build          # Build
cargo test           # Run tests (154 tests)
cargo fmt            # Format code
cargo clippy         # Lint
RUST_LOG=debug cargo run  # Run with debug logging
```

### Frontend

```bash
cd frontend

npm install          # Install dependencies
npm run dev          # Dev server with API proxy (port 5173 → backend 3000)
npm run build        # Production build to dist/
npx tsc --noEmit     # Type check
```

During development, Vite proxies `/api` requests to the backend at `http://localhost:3000`.

### Docker Build

```bash
docker compose build   # Build containers
docker compose up -d   # Start full stack
docker compose logs -f app  # Follow app logs
```

The Dockerfile uses a multi-stage build:
1. **Node (Alpine)** — builds the React frontend
2. **Rust (slim)** — compiles the backend binary, statically linking the C++ runtime that embedded DuckDB needs
3. **Chainguard `glibc-dynamic`** — minimal distroless runtime image, runs as non-root

## Agronomic Rules

TurfOps includes 14 rules that evaluate environmental conditions and generate actionable recommendations. Rules are divided into current-condition rules (using real-time sensor data) and forecast-based rules (using OpenWeatherMap data). Turf diseases are not handled by rules — see [Disease Risk](#disease-risk).

### Current-Condition Rules

#### Pre-Emergent Timing
**Purpose**: Prevent crabgrass before it germinates

Crabgrass seeds germinate when soil temperature at 2-4" depth reaches 55°F for 3+ consecutive days. Pre-emergent herbicides must be applied *before* germination begins.

| Condition | Severity | Action |
|-----------|----------|--------|
| 7-day soil avg 50-55°F | Advisory | Optimal window - apply pre-emergent |
| 7-day soil avg 55-60°F | Warning | Window narrowing - apply soon |
| 7-day soil avg 60-70°F | Critical | Window closing - apply immediately |

**Active**: February through May | **Products**: Prodiamine, dithiopyr, or pendimethalin at label rate. Water in within 24 hours.

#### Spring Nitrogen Timing
**Purpose**: Prevent damage from fertilizing too early in spring

| Condition | Severity | Action |
|-----------|----------|--------|
| Soil <50°F | Info | Wait - too cold for fertilizer |
| Soil 50-55°F | Info | Almost ready - continue waiting |
| Soil 55-65°F, no spring fert yet | Advisory | Ready for light spring nitrogen |
| Fertilized while soil <55°F | Warning | Applied too early - avoid more nitrogen |

**Key Points**: Wait until soil reaches 55°F (7-day average). Spring nitrogen should be light (0.5 lb N/1000 sqft) — save heavy feeding for fall.

#### Grub Control Timing
**Purpose**: Prevent grub damage through preventative insecticide

| Condition | Severity | Action |
|-----------|----------|--------|
| May 15 - Jul 4, soil 60-75°F | Advisory | Apply preventative grub control |
| <14 days remaining in window | Warning | Apply soon - window closing |
| Soil >75°F | Info | Late but may still be effective |

**Active**: May 15 through July 4 | **Products**: Chlorantraniliprole (GrubEx) or imidacloprid.

#### Fertilizer Stress Block
**Purpose**: Prevent fertilizer burn during heat or moisture stress

| Condition | Severity | Action |
|-----------|----------|--------|
| Ambient temp >85°F | Warning | Avoid nitrogen application |
| Ambient temp >90°F | Critical | Do NOT apply any fertilizer |
| Soil moisture <0.10 | Warning | Drought stress - irrigate first |
| Soil moisture <0.05 | Critical | Severe drought - delay fertilizer |
| Soil moisture >0.40 | Warning | Saturated - fertilizer will leach |

#### Broadleaf Herbicide Timing
**Purpose**: Target broadleaf weeds during optimal control windows

| Condition | Severity | Action |
|-----------|----------|--------|
| March, soil 45-55°F rising | Advisory | Spring window — target winter annuals |
| Late Sept - Oct, soil 50-65°F | Warning | Fall window — best for perennial weeds |

**Active**: March (spring) and September 20 - October 31 (fall). Blocked if overseeded within 60 days.

#### Mowing Height
**Purpose**: Seasonal mowing guidance for TTTF

| 7-Day Avg Temp | Height | Severity |
|-----------------|--------|----------|
| 50-75°F | 2.5-3.5" | Info |
| 75-85°F | 3-4" | Advisory |
| >85°F | 3.5-4" | Warning |

Never remove more than 1/3 of the blade at once.

#### Core Aeration
**Purpose**: Relieve soil compaction during peak recovery season

| Condition | Severity | Action |
|-----------|----------|--------|
| Aug 15 - Oct 15, soil 50-65°F, not aerated in 12+ months | Advisory | Aerate this fall |
| Clay/Clay Loam soil, not aerated in 12+ months | Warning | Annual aeration important |

**Active**: August 15 through October 15. Best combined with fall overseeding.

### Fall Program Rules

#### Fall Overseeding
**Purpose**: Thicken the lawn during optimal germination conditions

| Condition | Severity | Action |
|-----------|----------|--------|
| Aug 15 - Oct 31, soil 50-65°F | Advisory | Optimal overseeding window |
| Soil 55-62°F | Advisory (Peak) | Best germination temps |
| <21 days remaining, optimal temps | Warning | Seed soon - window closing |

**Seeding Rate**: 4 lbs per 1000 sqft for overseeding (8 lbs for bare soil).

#### Fall Fertilization Program
**Purpose**: Build root reserves for winter survival and spring green-up

| Phase | Timing | Nitrogen Rate | Purpose |
|-------|--------|---------------|---------|
| Early Fall | September | 0.5 lb N/1000 sqft | Recovery from summer stress |
| Mid Fall | October | 0.75 lb N/1000 sqft | Primary fall feeding (most important) |
| Late Fall | November | 1.0 lb N/1000 sqft | Winterizer - stores for spring |

### Forecast-Based Rules

These rules require OpenWeatherMap API integration (`OWM_API_KEY`).

#### Rain Delay
Prevents wasted chemical applications before rain.

| Condition | Severity | Action |
|-----------|----------|--------|
| Rain >0.1" expected in 24-48h, <50% prob | Advisory | Consider timing carefully |
| Rain in 24h, >50% probability | Warning | Delay applications if possible |
| Rain in 12h, >70% probability | Critical | Do NOT apply any products |

#### Irrigation Forecast
Recommends irrigation when drought conditions are developing.

| Condition | Severity | Action |
|-----------|----------|--------|
| No rain 5 days + moisture 0.15-0.20 | Advisory | Monitor and prepare to irrigate |
| No rain 5 days + moisture 0.10-0.15 | Warning | Irrigate within 1-2 days |
| No rain 5 days + moisture <0.10 | Critical | Water immediately |

#### Heat Stress Warning
Prepares for upcoming heat stress conditions.

| Forecasted Max Temp | Severity | Action |
|---------------------|----------|--------|
| 85-90°F in next 3 days | Advisory | Raise mowing height, water early |
| 90-95°F in next 3 days | Warning | Avoid fertilizer, skip mowing |
| >95°F in next 3 days | Critical | Accept dormancy, minimize all stress |

#### Optimal Application Window
Identifies the best days for chemical applications based on forecast (dry weather, moderate temps, low wind).

#### Soil Temperature Forecast
Proactive heads-up when the soil-temperature prediction model (air-to-soil regression on recent lake data plus the forecast) expects an agronomic threshold crossing soon, e.g. the pre-emergent window approaching.

## Disease Risk

Each disease is scored independently by its own model and then read off on a common **Low / Moderate / High / Severe** scale. The native scores (a probability, an index, a point score) are not comparable to each other, so there is deliberately no single blended "disease pressure" number.

| Disease | Model | Inputs | Tiers (Moderate / High / Severe) |
|---------|-------|--------|----------------------------------|
| **Brown patch** | Fidanza-Dernoeden E-index — `E = −21.5 + 0.15·RH + 1.4·Tmin − 0.033·Tmin²` (Phytopathology 86:385–390, 1996) | Daily min air temp (°C), daily mean RH | E ≥ 0 / ≥ 5 / ≥ 6 |
| **Dollar spot** | Smith-Kerns logistic — `logit = −11.4041 + 0.0894·MEANRH + 0.1932·MEANAT`, inactive outside 10–35°C (PLOS ONE 13(3):e0194216, 2018) | 5-day moving averages of daily mean air temp and RH | ≥ 10% / ≥ 20% (published action threshold) / ≥ 40% |
| **Pythium blight** | 0–5 score on the Nutter et al. criteria (Plant Disease 67:1126–1128, 1983): +1 max ≥ 86°F, +1 min ≥ 68°F, +1 each at 6 / 10 / 14 h of RH ≥ 90%; capped at 1 without a heat criterion | Daily max/min air temp, hours RH ≥ 90% | 2 / 3 / 4 |
| **Gray leaf spot** *(experimental)* | 0–100 suitability = temperature × leaf wetness, 3-day mean, July–October only (drivers per Uddin et al., Phytopathology 93:336–343, 2003) | Daily mean air temp, estimated leaf wetness | 25 / 50 / 75 |
| **Red thread** *(experimental)* | 0–100 suitability = cool temperature × leaf wetness, 5-day mean | Daily mean air temp, estimated leaf wetness, rainfall | 25 / 50 / 75 |

Brown patch and dollar spot are peer-reviewed, field-validated models. Pythium uses published thresholds combined into a heuristic score. Gray leaf spot and red thread have no validated turf forecasting model, so they are composite indices and are labeled **experimental** in the UI — use them alongside scouting.

**Lawn history modifies risk.** Overseeding within 60 days raises gray leaf spot one tier (seedlings are highly susceptible); no fertilizer in 60 days raises red thread one tier (it is a low-nitrogen disease). Only the headline tier is raised, never from Low, and the page says why. The daily chart stays weather-only.

**Where the data comes from.** Observed days are aggregated per local day from the lake's hourly silver layer (air temp, RH, precipitation). Leaf wetness is estimated as hours with RH ≥ 90% or measurable precipitation; dew point uses the Magnus formula. The lake trails real time by up to a day, so today's remaining hours and the 4-day outlook come from the OpenWeatherMap forecast. When too little of today is covered (under 12 hours), the headline reflects the most recent complete day and the page shows a note saying so. Without an OpenWeatherMap key there is no outlook.

### What to do: preventative and curative guidance

Every disease page turns its tier into an action:

| Tier | Action |
|------|--------|
| Low | No action |
| Moderate | Monitor — cultural practices only, with a heads-up if the 3-day outlook reaches High or Severe |
| High / Severe | Apply a preventative — or **Protected** if a logged fungicide still covers this disease |

- **Cultural practices** come first for every disease (nitrogen, irrigation timing, mowing, airflow).
- A **preventative program** and a **curative program** ("if you see symptoms") list FRAC classes with example products, efficacy, and interval. TurfOps cannot see your lawn, so the curative program is always shown for you to apply on your own judgment.
- **Protection window**: a logged fungicide that is at least *Good* on a disease counts as protection for 21 days (single-site systemic) or 14 days (contact / phosphonate), shortened by 7 days under Severe pressure. Efficacy is per disease — a strobilurin applied for brown patch does not count as dollar spot or Pythium protection.
- **Red thread** is treated with nitrogen, not fungicide.
- **No application rates are given.** Rates are product-specific; always read and follow the product label. Efficacy ratings summarize university extension guidance and are a starting point, not a prescription.

Diseases at High or Severe also appear as recommendations on the Dashboard and Recommendations page, generated from the same model output so the two always agree.

### FRAC Rotation System

TurfOps tracks fungicide application history and provides rotation-aware recommendations to prevent resistance development.

| FRAC Class | Type | Common Products |
|------------|------|-----------------|
| FRAC 1 | Thiophanates | thiophanate-methyl, Cleary's 3336 |
| FRAC 3 | DMIs/Triazoles | propiconazole, Banner MAXX, myclobutanil, Eagle |
| FRAC 4 | Phenylamides (Pythium) | mefenoxam, Subdue MAXX |
| FRAC 7 | SDHI | fluxapyroxad, Xzemplar, penthiopyrad, Velista |
| FRAC 11 | Strobilurins | azoxystrobin, Heritage, pyraclostrobin, Insignia |
| FRAC 12 | Phenylpyrroles | fludioxonil, Medallion |
| FRAC 14 | Aromatics | PCNB, Turfcide |
| FRAC 21 | QiI (Pythium) | cyazofamid, Segway |
| FRAC 28 | Carbamates (Pythium) | propamocarb, Banol |
| FRAC P07 | Phosphonates (Pythium, preventative only) | fosetyl-Al, Signature, phosphite |
| FRAC M3 | Multi-site | chlorothalonil, Daconil — not labeled for residential lawns in the US; tracked for protection, never suggested |
| FRAC M5 | Multi-site | mancozeb |

**Rotation**: for each disease, the suggested class is the most effective option for *that disease* that is not the single-site class you used last; the last-used class is tagged "rotate". A season-level warning appears after two consecutive applications of the same single-site class or three or more fungicide applications in a year. Multi-site fungicides (M3, M5) are excluded from rotation calculations (low resistance risk). FRAC classes are inferred from the product name on the logged application, so use a recognizable product or active-ingredient name.

## Lawn Profile Defaults

- **Location**: Media, PA (USDA Zone 7a)
- **Grass Type**: Turf Type Tall Fescue (TTTF)
- **NOAA Station**: PA Avondale (WBANNO 3761)

## Agronomic Thresholds (TTTF Zone 7a)

| Metric | Threshold | Meaning |
|--------|-----------|---------|
| Soil temp 10cm | 50-60°F | Pre-emergent window |
| Soil temp 10cm | 60-75°F | Grub control window |
| Ambient temp | >85°F | Fertilizer stress risk |
| Soil moisture | <0.10 | Irrigation needed |
| Soil moisture | >0.40 | Saturated - avoid fertilizer |
| Humidity | >80% | Reference line on the Environmental humidity chart |
| Brown patch E-index | ≥5 | Brown patch warning (High) |
| Dollar spot probability | ≥20% | Smith-Kerns action threshold (High) |

## License

Private project.
