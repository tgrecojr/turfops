# TurfOps Enhancement Roadmap

## Status Tracker

| # | Feature | Tier | Status | Notes |
|---|---------|------|--------|-------|
| 1 | Growing Degree Day (GDD) Tracking | 1 | Done | GDD base 50°F, crabgrass model, dashboard widget, pre-emergent rule integration |
| 2 | Soil Test Tracking & pH Management | 1 | Done | Soil Tests page, lime/sulfur rates by soil type, N-P-K + micronutrient recommendations |
| 3 | Annual Nitrogen Budget & Nutrient Tracking | 1 | Done | Running N budget with per-grass-type targets, N-P-K on applications, dashboard widget |
| 4 | Notifications / Push Alerts | 1 | Not Started | Email/webhook for Warning/Critical severity |
| 5 | Historical Trends & Year-over-Year Analytics | 1 | Done | 6 trend charts on Environmental page (7d/30d/90d), threshold reference lines |
| 6 | Product Database & Rate Calculator | 2 | Not Started | Common products with N-P-K, rate math |
| 7 | Mowing Log & 1/3 Rule Enforcement | 2 | Partial | Mowing log via ApplicationType; 1/3 rule enforcement deferred |
| 8 | Frost Date Integration & Season Boundaries | 2 | Not Started | First/last frost, deadline anchoring |
| 9 | Proactive Seasonal Plan / Program Builder | 2 | Done | Historical NOAA analysis, threshold crossings, 10 activities, timeline UI |
| 10 | Multi-Zone Support | 2 | Not Started | Multiple lawn profiles with independent recs |
| 11 | Soil Temperature Prediction Model | 3 | Done | Air-to-soil regression with lag selection, threshold-crossing predictions, dashboard widget + proactive rule |
| 12 | ET-Based Irrigation Intelligence | 3 | Not Started | Evapotranspiration model, smart watering recs |
| 13 | Photo Journal / Lawn Progress | 3 | Not Started | Periodic photos, visual progress tracking |
| 14 | Weed ID & Treatment Reference | 3 | Not Started | Weed → active ingredient → product mapping |
| 15 | Export & Annual Reporting | 3 | Not Started | PDF/CSV export, annual summary |
| 16 | Cost Tracking | 3 | Not Started | Per-application cost, annual spend dashboard |
| 17 | Per-Disease Risk Models & Management | 1 | Done | Brown patch (Fidanza E-index), dollar spot (Smith-Kerns), Pythium, gray leaf spot, red thread; Disease Risk page, dashboard widget, preventative/curative FRAC-aware guidance; replaced 5 heuristic disease rules |
| 18 | Same-Day Weather for Disease Risk | 2 | Not Started | Lake lags ~1 day; evaluate Open-Meteo (modeled hourly) or NWS API (measured obs) to fill today + outlook |
| 19 | Pre-Season DMI Window | 3 | Not Started | GDD-based (140–175 base 50°F from Feb 15) early-season dollar spot fungicide window in the seasonal plan |
| 20 | Improved Leaf Wetness Estimate | 3 | Not Started | Use the lake's unused solar radiation + surface temperature columns instead of the RH ≥ 90% proxy |

## Feature Details

### Tier 1 — High-Impact Gaps

#### 1. Growing Degree Day (GDD) Tracking
GDD (base 50°F for cool-season grass) is the gold standard for timing lawn care applications. Crabgrass germinates at ~200 GDD₅₀. The NOAA hourly temp data already exists to calculate this. Pre-emergent rules become significantly more precise: "GDD is at 145. Crabgrass germination expected in ~5 days."

**Includes:**
- GDD calculation from historical ambient temp data
- Running GDD accumulator (Jan 1 reset)
- Crabgrass germination model (200 GDD₅₀ threshold)
- Historical trend charts (soil temp, ambient temp, GDD accumulation over time)
- Year-over-year comparison capability
- Integration with pre-emergent rule for improved timing

#### 2. Soil Test Tracking & pH Management
Store soil test results (pH, P, K, Ca, Mg, organic matter, CEC, buffer pH), track trends over years, and calculate lime/sulfur application rates based on soil type and buffer pH.

#### 3. Annual Nitrogen Budget & Nutrient Tracking
TTTF in Zone 7a needs 3-4 lbs N/1000 sqft/year. Track running N budget to prevent over-fertilization. Requires knowing N-P-K ratios of applied products.

**Includes:**
- Product N-P-K storage on applications
- Running annual N/1000 sqft accumulator
- Budget remaining calculation (target - applied)
- Dashboard widget showing N budget status
- Warning when approaching/exceeding annual N target

#### 4. Notifications / Push Alerts
Email/webhook integration for Warning/Critical severity recommendations. Time-critical events: pre-emergent windows, frost warnings, disease outbreaks, rain delays.

#### 5. Historical Trends & Year-over-Year Analytics
Visualize environmental cache data: soil temp warming curves, moisture patterns, seasonal comparisons. Line charts with threshold bands overlaid.

### Tier 2 — Valuable Enhancements

#### 6. Product Database & Rate Calculator
Curated list of common lawn care products with N-P-K ratios, active ingredients, FRAC classes. Rate calculator: "You have Product X. For Y lbs N/1000 sqft on Z sqft lawn = W lbs of product."

#### 7. Mowing Log & 1/3 Rule Enforcement
Track mowing date and cut height. Enforce the 1/3 rule (never cut more than 1/3 of blade height). Mowing frequency analysis tied to growth conditions.

#### 8. Frost Date Integration & Season Boundaries
Configure first/last frost dates. Anchor critical deadlines: last safe overseed date, winterizer deadline, spring nitrogen start.

#### 9. Proactive Seasonal Plan / Program Builder
Generate personalized annual roadmap from rules engine knowledge projected forward using historical GDD/soil temp data.

#### 10. Multi-Zone Support
Multiple named lawn profiles with independent recommendation streams. Support distinct zones (front/back, sun/shade, different soil types).

### Tier 3 — Advanced / Differentiating

#### 11. Soil Temperature Prediction Model
Predict future soil temperatures from air temp correlation plus forecast data.

#### 12. ET-Based Irrigation Intelligence
Evapotranspiration model for precise watering recommendations (how much, how often).

#### 13. Photo Journal / Lawn Progress
Periodic photos of same areas for visual progress tracking. Future: image-based disease/weed identification.

#### 14. Weed ID & Treatment Reference
Reference guide mapping common weeds to active ingredients, products, and timing.

#### 15. Export & Annual Reporting
PDF/CSV export of application history, annual N applied, soil test trends, environmental data.

#### 16. Cost Tracking
Product cost per application, annual spend dashboard, cost per 1000 sqft metrics.

### Disease Risk

#### 17. Per-Disease Risk Models & Management
Each disease is scored by its own published or heuristic weather model and read off on a shared Low/Moderate/High/Severe scale, with an 11-day observed + forecast series, contributing factors, a "how this is calculated" breakdown, and a management plan (cultural practices, preventative and curative programs, rotation-aware class suggestion, protection window from logged fungicides). Inspired by the r/LawnAnswers Turf Tools disease pages, but using the published Smith-Kerns model and the app's own station data and application history. A symptoms-observed toggle was considered and deliberately left out — the curative program is always shown instead.

**Known limitations:** the fungicide efficacy matrix summarizes extension guidance and should be reviewed against the current Univ. of Kentucky PPA-1; gray leaf spot and red thread are experimental indices; leaf wetness is an RH-based estimate.

#### 18. Same-Day Weather for Disease Risk
The lake loads about once a day, so "today" relies on the remaining OpenWeatherMap 3-hourly forecast points and the headline falls back to yesterday late in the day. Open-Meteo (free, keyless, hourly with `past_days`) would give gapless modeled hours; the NWS API would give *measured* same-day observations. Either is a new external dependency — adopt only if the lag becomes a real limitation.

#### 19. Pre-Season DMI Window
As described by the r/LawnAnswers Turf Tools "Pre-Season DMI Window" page, which attributes it to Michigan State's GDD Tracker (Dwyer & Vargas 2004) — verify against the MSU source before building: accumulate base-50°F GDD from February 15; the preventative DMI window for dollar spot opens at 140 and closes at 175. The gold layer already has daily `gdd50`, so this is mostly a seasonal-plan activity with a different start date.

#### 20. Improved Leaf Wetness Estimate
The silver layer carries `solar_rad_wm2` and `surface_temp_c`, which nothing reads today. Leaf wetness (dew) forms when the surface cools to the dew point at night and burns off with morning sun, so these could replace the crude RH ≥ 90% proxy used by the gray leaf spot and red thread indices.
