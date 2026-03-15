# TurfOps Enhancement Roadmap

## Status Tracker

| # | Feature | Tier | Status | Notes |
|---|---------|------|--------|-------|
| 1 | Growing Degree Day (GDD) Tracking | 1 | Done | GDD base 50°F, crabgrass model, dashboard widget, pre-emergent rule integration |
| 2 | Soil Test Tracking & pH Management | 1 | Not Started | Store results, calculate lime/sulfur rates |
| 3 | Annual Nitrogen Budget & Nutrient Tracking | 1 | Done | Running N budget with per-grass-type targets, N-P-K on applications, dashboard widget |
| 4 | Notifications / Push Alerts | 1 | Not Started | Email/webhook for Warning/Critical severity |
| 5 | Historical Trends & Year-over-Year Analytics | 1 | Done | 6 trend charts on Environmental page (7d/30d/90d), threshold reference lines |
| 6 | Product Database & Rate Calculator | 2 | Not Started | Common products with N-P-K, rate math |
| 7 | Mowing Log & 1/3 Rule Enforcement | 2 | Partial | Mowing log via ApplicationType; 1/3 rule enforcement deferred |
| 8 | Frost Date Integration & Season Boundaries | 2 | Not Started | First/last frost, deadline anchoring |
| 9 | Proactive Seasonal Plan / Program Builder | 2 | Done | Historical NOAA analysis, threshold crossings, 10 activities, timeline UI |
| 10 | Multi-Zone Support | 2 | Not Started | Multiple lawn profiles with independent recs |
| 11 | Soil Temperature Prediction Model | 3 | Not Started | Predict future soil temps from air temp correlation |
| 12 | ET-Based Irrigation Intelligence | 3 | Not Started | Evapotranspiration model, smart watering recs |
| 13 | Photo Journal / Lawn Progress | 3 | Not Started | Periodic photos, visual progress tracking |
| 14 | Weed ID & Treatment Reference | 3 | Not Started | Weed → active ingredient → product mapping |
| 15 | Export & Annual Reporting | 3 | Not Started | PDF/CSV export, annual summary |
| 16 | Cost Tracking | 3 | Not Started | Per-application cost, annual spend dashboard |

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
