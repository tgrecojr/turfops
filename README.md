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
- **Weather Forecast**: OpenWeatherMap 5-day/3-hour forecast API (optional, enables forecast-based rules)

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

TurfOps includes a rules engine that evaluates environmental conditions and generates actionable recommendations. Rules are divided into two categories: current-condition rules (using real-time sensor data) and forecast-based rules (using OpenWeatherMap data).

### Current-Condition Rules

These rules evaluate real-time soil and ambient conditions from NOAA and Home Assistant sensors.

#### Pre-Emergent Timing
**Purpose**: Prevent crabgrass before it germinates

Crabgrass seeds germinate when soil temperature at 2-4" depth reaches 55°F for 3+ consecutive days. Pre-emergent herbicides must be applied *before* germination begins.

| Condition | Severity | Action |
|-----------|----------|--------|
| 7-day soil avg 50-55°F | Advisory | Optimal window - apply pre-emergent |
| 7-day soil avg 55-60°F | Warning | Window narrowing - apply soon |
| 7-day soil avg 60-70°F | Critical | Window closing - apply immediately |

**Active**: February through May (spring only)
**Products**: Prodiamine, dithiopyr, or pendimethalin at label rate. Water in within 24 hours.

---

#### Spring Nitrogen Timing
**Purpose**: Prevent damage from fertilizing too early in spring

A common mistake is fertilizing as soon as you see green. This forces top growth before roots wake up, depleting carbohydrate reserves and creating shallow roots that struggle in summer.

| Condition | Severity | Action |
|-----------|----------|--------|
| Soil <50°F | Info | Wait - too cold for fertilizer |
| Soil 50-55°F | Info | Almost ready - continue waiting |
| Soil 55-65°F, no spring fert yet | Advisory | Ready for light spring nitrogen |
| Fertilized while soil <55°F | Warning | Applied too early - avoid more nitrogen |

**Key Points**:
- Wait until soil reaches 55°F (7-day average)
- Wait until after 2-3 mowings
- Spring nitrogen should be LIGHT (0.5 lb N/1000 sqft) - save heavy feeding for fall
- Pre-emergent timing (50-55°F) comes BEFORE fertilization

**Explanation**: Cool-season grass breaks dormancy using stored carbohydrates, not soil nutrients. Early nitrogen forces weak top growth at the expense of root development, weakening the plant heading into summer stress.

---

#### Grub Control Timing
**Purpose**: Prevent grub damage through preventative insecticide

Japanese beetle and other grub larvae are most vulnerable when adults are laying eggs and larvae are feeding near the soil surface.

| Condition | Severity | Action |
|-----------|----------|--------|
| May 15 - Jul 4, soil 60-75°F | Advisory | Apply preventative grub control |
| <14 days remaining in window | Warning | Apply soon - window closing |
| Soil >75°F | Info | Late but may still be effective |

**Active**: May 15 through July 4
**Products**: Chlorantraniliprole (GrubEx) or imidacloprid. Water in with 0.5" irrigation within 24 hours.

---

#### Fertilizer Stress Block
**Purpose**: Prevent fertilizer burn during heat or moisture stress

Cool-season grasses experience stress during extreme heat and will burn if fertilized. Saturated soil causes fertilizer runoff; drought-stressed turf can't absorb nutrients.

| Condition | Severity | Action |
|-----------|----------|--------|
| Ambient temp >85°F | Warning | Avoid nitrogen application |
| Ambient temp >90°F | Critical | Do NOT apply any fertilizer |
| Soil moisture <0.10 | Warning | Drought stress - irrigate first |
| Soil moisture <0.05 | Critical | Severe drought - delay fertilizer |
| Soil moisture >0.40 | Warning | Saturated - fertilizer will leach |

**Explanation**: Tall Fescue and other cool-season grasses may go partially dormant above 85°F. Nitrogen forces top growth at the expense of roots, weakening the plant during stress.

---

#### Fungicide Disease Risk
**Purpose**: Alert when conditions favor brown patch and other fungal diseases

Brown patch (Rhizoctonia solani) thrives in hot, humid conditions with night temperatures above 65°F.

| Condition | Severity | Action |
|-----------|----------|--------|
| Humidity >80% + temp >70°F | Advisory | Monitor for symptoms |
| Sustained humidity >75% (7-day avg) | Warning | Consider preventative fungicide |
| Humidity >90% + temp >80°F + sustained | Critical | Apply fungicide immediately |

**Symptoms**: Circular patches of tan/brown turf with dark "smoke ring" border visible in morning dew.
**Products**: Azoxystrobin, propiconazole, or thiophanate-methyl. Avoid evening irrigation.

---

### Fall Program Rules

Fall is THE most important season for cool-season lawn care. These rules guide the two critical fall activities: overseeding and fertilization.

#### Fall Overseeding
**Purpose**: Thicken the lawn by seeding during optimal germination conditions

Tall Fescue doesn't spread via rhizomes or stolons - overseeding is the ONLY way to thicken a TTTF lawn and fill bare spots. Fall is the best time because soil is warm (fast germination), air is cool (less seedling stress), weed competition is minimal, and fall rains provide moisture.

| Condition | Severity | Action |
|-----------|----------|--------|
| Aug 15 - Oct 31, soil 50-65°F | Advisory | Optimal overseeding window |
| Soil 55-62°F | Advisory (Peak) | Best germination temps |
| <21 days remaining, optimal temps | Warning | Seed soon - window closing |
| Soil >65°F (early Sept) | Info | Wait for cooler temps |
| Soil <50°F | Warning | Germination will be slow - seed NOW if planned |

**Active**: August 15 through October 31
**Seeding Rate**: 4 lbs per 1000 sqft for overseeding (8 lbs for bare soil)
**Preparation**: Mow low (2"), dethatch or core aerate for seed-to-soil contact
**Watering**: Keep soil moist with light watering 2-3x daily for 14 days
**Traffic**: Avoid foot traffic for 3-4 weeks

---

#### Fall Fertilization Program
**Purpose**: Build root reserves for winter survival and spring green-up

Fall is when TTTF does most of its root development. While top growth slows, roots are actively growing and storing carbohydrates. Fall nitrogen has more impact on lawn health than any other seasonal feeding.

| Phase | Timing | Nitrogen Rate | Purpose |
|-------|--------|---------------|---------|
| Early Fall | September | 0.5 lb N/1000 sqft | Recovery from summer stress |
| Mid Fall | October | 0.75 lb N/1000 sqft | Primary fall feeding (MOST important!) |
| Late Fall | November | 1.0 lb N/1000 sqft | "Winterizer" - stores for spring |

| Condition | Severity | Action |
|-----------|----------|--------|
| September, no fall fert yet | Advisory | Time for early fall feeding |
| October, <2 fall apps | Advisory/Warning | Primary feeding - don't miss this! |
| November, <3 fall apps | Advisory | Apply winterizer before ground freezes |
| Missed all fall apps by Nov | Warning | At minimum, apply winterizer |

**Key Points**:
- Total fall nitrogen: 2-2.5 lbs N per 1000 sqft across all applications
- Space applications 3-4 weeks apart
- Mid-fall (October) application has the most impact on spring lawn quality
- Apply winterizer even if grass looks dormant - roots are still working
- Soil temp 45-65°F is optimal for root uptake

---

### Forecast-Based Rules

These rules require OpenWeatherMap API integration and use the 5-day weather forecast to predict future conditions.

#### Rain Delay
**Purpose**: Prevent wasted chemical applications before rain

Fertilizers, herbicides, and fungicides need time to be absorbed before rain. Rain within 24-48 hours washes products away, reducing effectiveness and potentially polluting waterways.

| Condition | Severity | Action |
|-----------|----------|--------|
| Rain >0.1" expected in 24-48h, <50% prob | Advisory | Consider timing carefully |
| Rain in 24h, >50% probability | Warning | Delay applications if possible |
| Rain in 12h, >70% probability | Critical | Do NOT apply any products |

**Explanation**: Most lawn chemicals need 4-6 hours to dry and begin absorption. Allow at least 24 hours before rain for best results, 48 hours for herbicides.

---

#### Irrigation Forecast
**Purpose**: Recommend irrigation when drought conditions are developing

When soil moisture is low and no rain is forecasted, supplemental irrigation prevents drought stress, thinning, and weed invasion.

| Condition | Severity | Action |
|-----------|----------|--------|
| No rain 5 days + moisture 0.15-0.20 | Advisory | Monitor and prepare to irrigate |
| No rain 5 days + moisture 0.10-0.15 | Warning | Irrigate within 1-2 days |
| No rain 5 days + moisture <0.10 | Critical | Water immediately |

**Watering Guidelines**: Tall Fescue needs 1-1.5" per week during growing season. Water deeply (to 6") in early morning (5-9 AM) to encourage deep roots and minimize disease.

---

#### Heat Stress Warning
**Purpose**: Prepare for upcoming heat stress conditions

Cool-season grasses struggle above 85°F. Photosynthesis slows, root growth stops, and the grass may enter summer dormancy above 90°F.

| Forecasted Max Temp | Severity | Action |
|---------------------|----------|--------|
| 85-90°F in next 3 days | Advisory | Raise mowing height, water early |
| 90-95°F in next 3 days | Warning | Avoid fertilizer, skip mowing |
| >95°F in next 3 days | Critical | Accept dormancy, minimize all stress |

**Recommendations**: Raise mowing height to 3.5-4" (4"+ for extreme heat). Taller grass shades the crown and soil, reducing heat stress. Never mow during peak heat.

---

#### Optimal Application Window
**Purpose**: Identify the best days for chemical applications

Good application conditions include dry weather, moderate temperatures, low wind (to prevent drift), and moderate humidity.

| Condition | Score Factor |
|-----------|--------------|
| No rain 24h before AND 48h after | Required |
| Temperature 50-80°F | Required |
| Wind <10 mph | Preferred |
| Humidity <85% | Preferred |
| Temperature 55-75°F | Ideal |

**Output**: When a good window exists, provides an Info-level recommendation identifying the best day with specific conditions. Helps plan applications around weather.

---

#### Disease Pressure Forecast
**Purpose**: Predict elevated fungal disease risk from upcoming weather

Uses forecast data to predict when conditions will favor brown patch, dollar spot, and other fungal diseases *before* they occur.

| Condition | Risk Factor |
|-----------|-------------|
| Night temps >65°F (brown patch trigger) | +1 |
| Day temps 75-90°F with high humidity | +1 |
| Sustained humidity >80% for 2+ days | +1 per day |
| Rain followed by warm humid conditions | +1 |

| Combined Risk Score | Severity | Action |
|---------------------|----------|--------|
| 2-3 | Advisory | Monitor, prepare fungicide |
| 3-4 | Warning | Consider preventative application |
| 5+ or current + forecast high | Critical | Apply fungicide now |

**Target Diseases**: Brown patch (warm nights + humidity), Dollar spot (warm days + humidity + dew), Pythium blight (hot + wet).
**Prevention**: Water only in early morning, reduce nitrogen, consider preventative fungicide before conditions deteriorate.

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
