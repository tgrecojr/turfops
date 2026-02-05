# TurfOps

A Rust-based TUI application for tracking lawn care activities and providing data-driven agronomic recommendations.

## Features

- **Application Tracking**: Log fertilizer, pre-emergent, fungicide, and other lawn treatments
- **Environmental Data**: Real-time soil temperature, moisture, and ambient conditions
- **Smart Recommendations**: Data-driven alerts for optimal treatment timing
- **Calendar View**: Visualize application history and upcoming windows
- **Rules Engine**: Agronomic rules for cool-season grass (TTTF) in Zone 7a

## Data Sources

- **Ambient Conditions**: Patio Prometheus sensor via Home Assistant (local, real-time)
- **Soil Conditions**: NOAA USCRN data via SoilData PostgreSQL (authoritative soil data)
- **Precipitation**: NOAA measured values

### Related Projects

- **[SoilData](https://github.com/tgrecojr/soildata)**: Processes and stores NOAA USCRN hourly soil temperature and moisture data in PostgreSQL. TurfOps queries this database for authoritative soil conditions used in agronomic rule evaluation.

## Requirements

- Rust 1.70+
- PostgreSQL (for SoilData NOAA data)
- Home Assistant with Prometheus integration (optional, for ambient sensors)

## Installation

```bash
cargo build --release
```

## Configuration

1. Copy configuration examples:
   ```bash
   cp .env.example .env
   cp config/config.yaml.example config/config.yaml
   ```

2. Edit `.env` with your database credentials
3. Edit `config/config.yaml` for lawn profile and sensor settings

## Usage

```bash
cargo run
```

### Navigation

| Key | Action |
|-----|--------|
| `1` | Dashboard |
| `2` | Calendar |
| `3` | Applications |
| `4` | Environmental Data |
| `5` | Recommendations |
| `s` | Settings |
| `a` | Quick Add Application |
| `r` | Refresh Data |
| `q` | Quit |

## Agronomic Rules

| Rule | Trigger | Action |
|------|---------|--------|
| Pre-Emergent | 7-day avg soil temp 50-60°F | Apply before crabgrass germination |
| Grub Control | May 15 - July 4, soil temp 60-75°F | Preventative treatment window |
| Fertilizer Block | Ambient >85°F, moisture <0.10 or >0.40 | Avoid N application during stress |
| Fungicide Risk | Humidity >80% + ambient >70°F sustained | Brown patch prevention |
| Fall Overseeding | Soil temp 50-65°F (late Aug - Oct) | TTTF germination window |
| Frost Warning | Ambient <32°F | Avoid turf traffic |

## Lawn Profile

Default configuration:
- **Location**: Media, PA (USDA Zone 7a)
- **Grass Type**: Turf Type Tall Fescue (TTTF)
- **NOAA Station**: PA Avondale (WBANNO 3761)

## Development

```bash
# Run with debug logging
RUST_LOG=debug cargo run

# Run tests
cargo test

# Check formatting
cargo fmt --check

# Run linter
cargo clippy

# Run pre-commit checks manually
./scripts/pre-commit-checks.sh
```

### Pre-commit Hooks

Git pre-commit hooks are configured to run automatically on commit:
- Secret detection (blocks .env commits)
- Code formatting check (`cargo fmt`)
- Linting (`cargo clippy`) - errors fail, warnings allowed
- Unit tests (`cargo test`)
- Security audit (`cargo audit` if installed)

Install `cargo-audit` for vulnerability scanning:
```bash
cargo install cargo-audit
```

### Scaffolded Features

The codebase includes scaffolded code for planned features (may show as dead_code warnings):

| Module | Purpose | Status |
|--------|---------|--------|
| `db/queries.rs` | Full CRUD for applications, settings, cache management | Partially used |
| `logic/calculations.rs` | GDD calculation, averages, precipitation totals | Planned |
| `logic/data_sync.rs` | Background refresh, connection status | Partially used |
| `ui/components/input.rs` | Text input and select widgets for forms | Planned |
| `ui/screens/settings.rs` | Enum option constants for dropdowns | Planned |
| `ui/theme.rs` | Extended color palette and styles | Partially used |
