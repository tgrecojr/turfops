# TurfOps - Lawn Care Management Web Application

## Overview

Containerized web application for tracking lawn care activities with data-driven agronomic recommendations. Rust/Axum backend serves a React SPA frontend. Integrates with SoilData PostgreSQL (NOAA USCRN hourly data), Home Assistant (local patio sensors), and OpenWeatherMap (forecast). Deployed via Docker Compose.

## Tech Stack

- Backend: Rust + Axum + sqlx (PostgreSQL)
- Frontend: React 19 + TypeScript + Vite
- Database: PostgreSQL 16 (app data)
- External Data: SoilData PostgreSQL (NOAA), Home Assistant API, OpenWeatherMap API
- Deployment: Docker Compose (app + PostgreSQL)
- Async Runtime: Tokio

## Commands

### Backend
- `cd backend && cargo build` — Build backend
- `cd backend && cargo test` — Run tests (44 tests)
- `cd backend && cargo fmt` — Format code
- `cd backend && cargo clippy` — Run linter
- `cd backend && cargo run` — Run API server (needs PostgreSQL)

### Frontend
- `cd frontend && npm install` — Install dependencies
- `cd frontend && npm run dev` — Dev server with API proxy (port 5173)
- `cd frontend && npm run build` — Production build to dist/
- `cd frontend && npx tsc --noEmit` — Type check

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
│       ├── state.rs             # AppState (pool, sync, rules engine)
│       ├── api/                 # Route handlers (16 endpoints)
│       ├── db/                  # PostgreSQL pool, queries, migrations
│       ├── models/              # Data structures (shared with rules)
│       ├── logic/               # Data sync + 18 agronomic rules + GDD computation + seasonal plan
│       └── datasources/         # SoilData, HomeAssistant, OpenWeatherMap
├── frontend/
│   └── src/
│       ├── App.tsx              # React Router, 7 routes
│       ├── api/client.ts        # Fetch wrapper for all API endpoints
│       ├── types/index.ts       # TypeScript interfaces matching Rust models
│       ├── pages/               # Dashboard, Calendar, Applications, Environmental, Recommendations, SeasonalPlan, Settings
│       └── components/          # Layout, Gauge, AlertCard, TrendChart, GddWidget, NitrogenBudgetWidget
├── Dockerfile                   # Multi-stage: Node → Rust → slim runtime
└── docker-compose.yml           # app + PostgreSQL 16
```

## API Endpoints

| Method | Path | Purpose |
|--------|------|---------|
| GET | /api/v1/health | Connection status |
| GET | /api/v1/dashboard | Composite dashboard data |
| GET/PUT | /api/v1/profile | Lawn profile CRUD |
| GET/POST | /api/v1/applications | List/create applications |
| DELETE | /api/v1/applications/:id | Delete application |
| GET | /api/v1/applications/calendar | Calendar view |
| GET | /api/v1/environmental | Environmental data (demand-driven refresh) |
| POST | /api/v1/environmental/refresh | Force data refresh |
| GET | /api/v1/recommendations | Active recommendations |
| PATCH | /api/v1/recommendations/:id | Mark addressed/dismissed |
| GET | /api/v1/gdd | GDD accumulation + crabgrass germination model |
| GET | /api/v1/historical | Time-series environmental data (7d/30d/90d) |
| GET | /api/v1/nitrogen-budget | Annual nitrogen budget vs grass-type target |
| GET | /api/v1/seasonal-plan | Seasonal plan with predicted activity windows |

## Data Sources

- **Ambient (temp/humidity)**: Home Assistant API → patio sensor
- **Soil (temp/moisture)**: SoilData PostgreSQL → NOAA USCRN PA Avondale (WBANNO 3761)
- **Precipitation**: SoilData PostgreSQL → NOAA measured values
- **Forecast**: OpenWeatherMap API (5-day forecast)

## Key Patterns

- Demand-driven data refresh: sensors stale after 5min, forecast after 30min. Zero API calls when idle.
- 18 agronomic rules are pure functions — no IO, no UI dependencies
- Recommendation state (addressed/dismissed) tracked in-memory (resets on restart)
- All temperatures stored in Fahrenheit (convert from Celsius at ingestion)
- Axum serves React SPA static files with fallback to index.html for client-side routing
- Seasonal plan uses historical NOAA soil temp data (up to 10 years) to predict activity windows via threshold crossing analysis; crossings cached in DB for fast subsequent loads

## Environment Variables

See `backend/.env.example` for full list:
- `DATABASE_HOST`, `DATABASE_PORT`, `DATABASE_NAME`, `DATABASE_USER`, `DATABASE_PASSWORD` — App PostgreSQL connection
- `SOILDATA_DB_*` — External NOAA data PostgreSQL
- `HA_URL`, `HA_TOKEN` — Home Assistant connection
- `OWM_API_KEY` — OpenWeatherMap API key
- `LAWN_*` — Default lawn profile settings

## Agronomic Thresholds (TTTF Zone 7a)

| Metric | Threshold | Meaning |
|--------|-----------|---------|
| Soil temp 10cm | 50-60°F | Pre-emergent window |
| Soil temp 10cm | 60-75°F | Grub control window |
| Ambient temp | >85°F | Fertilizer stress risk |
| Soil moisture | <0.10 | Irrigation needed |
| Soil moisture | >0.40 | Saturated - avoid fertilizer |
| Humidity | >80% | Disease risk |
