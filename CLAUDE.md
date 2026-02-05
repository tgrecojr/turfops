# TurfOps - Lawn Care Management TUI

## Overview

Rust TUI application for tracking lawn care activities with data-driven agronomic recommendations. Integrates with SoilData PostgreSQL (NOAA USCRN hourly data) and Home Assistant Prometheus (local patio sensors).

## Tech Stack

- Language: Rust
- TUI Framework: Ratatui + Crossterm
- Local Storage: SQLite (rusqlite)
- External Data: PostgreSQL (sqlx), Prometheus HTTP API (reqwest)
- Async Runtime: Tokio

## Commands

- `cargo build` — Build the application
- `cargo run` — Run the TUI
- `cargo test` — Run tests
- `cargo fmt` — Format code
- `cargo clippy` — Run linter

## Architecture

```
src/
├── main.rs              # Entry point, terminal setup
├── app.rs               # Screen state management
├── config.rs            # Configuration loading
├── error.rs             # Custom error types
├── db/                  # SQLite layer
├── models/              # Data structures
├── logic/rules/         # Agronomic rules engine
├── datasources/         # External data clients
└── ui/                  # TUI screens and components
```

## Data Sources

- **Ambient (temp/humidity)**: Prometheus → Home Assistant patio sensor
- **Soil (temp/moisture)**: SoilData PostgreSQL → NOAA USCRN PA Avondale (WBANNO 3761)
- **Precipitation**: SoilData PostgreSQL → NOAA measured values

## Key Patterns

- Screen state management follows `runlogger/src/app.rs` pattern
- PostgreSQL queries follow `soildata/src/db/repository.rs` pattern
- All temperatures stored in Fahrenheit (convert from Celsius at ingestion)

## Environment Variables

See `.env.example`:
- `SOILDATA_DB_*` — PostgreSQL connection for NOAA data
- `PROMETHEUS_URL` — Home Assistant Prometheus endpoint

## Agronomic Thresholds (TTTF Zone 7a)

| Metric | Threshold | Meaning |
|--------|-----------|---------|
| Soil temp 10cm | 50-60°F | Pre-emergent window |
| Soil temp 10cm | 60-75°F | Grub control window |
| Ambient temp | >85°F | Fertilizer stress risk |
| Soil moisture | <0.10 | Irrigation needed |
| Soil moisture | >0.40 | Saturated - avoid fertilizer |
| Humidity | >80% | Disease risk |
