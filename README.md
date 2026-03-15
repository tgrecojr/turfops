# TurfOps

A containerized web application for tracking lawn care activities and providing data-driven agronomic recommendations. Built with a Rust/Axum backend serving a React SPA frontend, deployed via Docker Compose.

## Features

- **Application Tracking**: Log fertilizer, pre-emergent, fungicide, mowing, and other lawn treatments
- **Environmental Data**: Real-time soil temperature, moisture, and ambient conditions from multiple sources
- **Smart Recommendations**: 18 agronomic rules provide data-driven alerts for optimal treatment timing
- **Calendar View**: Visualize application history and seasonal plan activity windows with colored indicators
- **Seasonal Plan Integration**: Calendar overlays predicted activity windows from the seasonal plan alongside actual applications
- **FRAC Rotation**: Fungicide resistance management with automatic class rotation recommendations
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
│  │  └─────────────┘   │  18 agronomic rules    │  │  │
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
   SoilData PG         Home Assistant    OpenWeatherMap
   (NOAA USCRN)        (patio sensors)    (forecast)
```

## Tech Stack

- **Backend**: Rust + Axum + sqlx (PostgreSQL)
- **Frontend**: React 19 + TypeScript + Vite
- **Database**: PostgreSQL 16 (app data)
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
- (Optional) [SoilData](https://github.com/tgrecojr/soildata) PostgreSQL database for NOAA soil data
- (Optional) Home Assistant instance with temperature/humidity sensors
- (Optional) OpenWeatherMap API key for forecast-based rules

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

### SoilData (NOAA USCRN)

Connection to an external PostgreSQL database running [SoilData](https://github.com/tgrecojr/soildata), which provides hourly NOAA USCRN soil temperature and moisture data.

| Variable | Description | Default |
|----------|-------------|---------|
| `SOILDATA_DB_HOST` | SoilData PostgreSQL host | `host.docker.internal` |
| `SOILDATA_DB_PORT` | SoilData PostgreSQL port | `5432` |
| `SOILDATA_DB_NAME` | SoilData database name | `uscrn` |
| `SOILDATA_DB_USER` | SoilData database user | `postgres` |
| `SOILDATA_DB_PASSWORD` | SoilData database password | *(empty)* |
| `NOAA_STATION_WBANNO` | NOAA USCRN station ID | `3761` (PA Avondale) |

> **Tip**: When running Docker Compose on macOS/Windows, `host.docker.internal` resolves to the host machine, so a locally-running SoilData PostgreSQL is reachable at the default.

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

Enables forecast-based rules (rain delay, heat stress warnings, optimal application windows, disease pressure forecast).

| Variable | Description | Default |
|----------|-------------|---------|
| `OWM_API_KEY` | OpenWeatherMap API key | *(empty — disabled if not set)* |
| `OWM_LATITUDE` | Location latitude | *(none)* |
| `OWM_LONGITUDE` | Location longitude | *(none)* |
| `OWM_ENABLED` | Enable/disable OWM integration | `true` |

Sign up for a free API key at [openweathermap.org](https://openweathermap.org/api). The free tier (1,000 calls/day) is more than sufficient.

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

# SoilData PostgreSQL (running on host machine)
SOILDATA_DB_HOST=host.docker.internal
SOILDATA_DB_PORT=5432
SOILDATA_DB_NAME=uscrn
SOILDATA_DB_USER=postgres
SOILDATA_DB_PASSWORD=

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

# Logging
RUST_LOG=info
```

## Data Sources

| Source | Data Provided | Connection |
|--------|--------------|------------|
| **SoilData PostgreSQL** | Soil temperature (5/10/20/50/100cm), soil moisture, precipitation | External PostgreSQL via `SOILDATA_DB_*` vars |
| **Home Assistant** | Ambient temperature, humidity (patio sensor) | REST API via `HA_URL` + `HA_TOKEN` |
| **OpenWeatherMap** | 5-day/3-hour forecast (temp, rain, humidity, wind) | REST API via `OWM_API_KEY` |

### Related Projects

- **[SoilData](https://github.com/tgrecojr/soildata)**: Processes and stores NOAA USCRN hourly soil temperature and moisture data in PostgreSQL. TurfOps queries this database for authoritative soil conditions used in agronomic rule evaluation.

## API Endpoints

| Method | Path | Purpose |
|--------|------|---------|
| `GET` | `/api/v1/health` | Connection status for all datasources |
| `GET` | `/api/v1/dashboard` | Composite dashboard (profile, env summary, alerts, recent apps) |
| `GET` | `/api/v1/profile` | Current lawn profile |
| `PUT` | `/api/v1/profile` | Update lawn profile |
| `GET` | `/api/v1/applications?type=X` | List applications (optional type filter) |
| `POST` | `/api/v1/applications` | Create new application |
| `DELETE` | `/api/v1/applications/:id` | Delete application |
| `GET` | `/api/v1/applications/calendar?year=Y&month=M` | Applications grouped by date |
| `GET` | `/api/v1/environmental` | Environmental data with demand-driven refresh |
| `POST` | `/api/v1/environmental/refresh` | Force immediate data refresh |
| `GET` | `/api/v1/recommendations` | Active recommendations from rules engine |
| `PATCH` | `/api/v1/recommendations/:id` | Mark recommendation addressed/dismissed |

## Pages

| Page | Description |
|------|-------------|
| **Dashboard** | Gauges for soil temp, ambient temp, humidity, and soil moisture. Active alerts and recent applications. Auto-refreshes every 30 seconds. |
| **Applications** | Filterable table of all lawn treatments including mowing. Add new applications with type, product, rate, and notes. |
| **Calendar** | Month grid view with colored dots for applications and status-colored bars for seasonal plan activity windows. Click any date to see details for both. |
| **Environmental** | Detailed sensor data, soil depth readings, 7-day trends and averages. |
| **Recommendations** | Active recommendations from the rules engine. Mark as addressed or dismiss. |
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
cargo test           # Run tests (30 tests)
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
1. **Node 20 Alpine** — builds the React frontend
2. **Rust 1.88** — compiles the backend binary
3. **Debian Bookworm slim** — minimal runtime image

## Agronomic Rules

TurfOps includes 18 rules that evaluate environmental conditions and generate actionable recommendations. Rules are divided into current-condition rules (using real-time sensor data) and forecast-based rules (using OpenWeatherMap data).

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

#### Fungicide Disease Risk
**Purpose**: Alert when conditions favor brown patch and other fungal diseases

| Condition | Severity | Action |
|-----------|----------|--------|
| Humidity >80% + temp >70°F, night >60°F | Advisory | Monitor for symptoms |
| Night temp >65°F or humidity >90% + sustained | Warning | Consider preventative fungicide |
| Night >70°F + day >90°F + sustained humidity | Critical | Apply fungicide immediately |

Recommendations are FRAC-aware — see [FRAC Rotation System](#frac-rotation-system).

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

#### Disease Pressure Forecast
Predicts elevated fungal disease risk from upcoming weather patterns.

#### Gray Leaf Spot
Alerts when conditions favor this destructive TTTF disease. **Active**: July-September. FRAC-aware — rotates away from FRAC 11 if recently used.

#### Pythium Blight
Alerts when conditions favor this fast-moving disease. **Active**: June-September. **Products**: Mefenoxam (FRAC 4) or fosetyl-Al (FRAC P07).

#### Red Thread
Identifies nitrogen deficiency through red thread symptoms. **Active**: March-May and September-November. Managed by fertilizing, not fungicide.

### FRAC Rotation System

TurfOps tracks fungicide application history and provides rotation-aware recommendations to prevent resistance development.

| FRAC Class | Type | Common Products |
|------------|------|-----------------|
| FRAC 1 | Thiophanates | thiophanate-methyl, Cleary's 3336 |
| FRAC 3 | DMIs/Triazoles | propiconazole, Banner MAXX, myclobutanil, Eagle |
| FRAC 7 | SDHI | fluxapyroxad, Xzemplar, penthiopyrad, Velista |
| FRAC 11 | Strobilurins | azoxystrobin, Heritage, pyraclostrobin, Insignia |
| FRAC 12 | Phenylpyrroles | fludioxonil, Medallion |
| FRAC 14 | Aromatics | PCNB, Turfcide |
| FRAC M3 | Multi-site | chlorothalonil, Daconil |
| FRAC M5 | Multi-site | mancozeb |

**Rotation order**: FRAC 11 → FRAC 3 → FRAC 1 → FRAC 7. Multi-site fungicides (M3, M5) are excluded from rotation calculations (low resistance risk).

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
| Humidity | >80% | Disease risk |

## License

Private project.
