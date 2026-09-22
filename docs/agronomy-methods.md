# TurfOps Agronomy Methods

The thresholds, models and data rules behind TurfOps' advice, in one place. The MCP
server serves this file verbatim as `turfops://docs/agronomy`; the seeding and
pre-emergent method is in `turfops://docs/timing-windows`. **Keep this file in sync when
a threshold, tier or rule in the code changes.**

## The lawn and its data

- Cool-season lawn (tall fescue, Kentucky bluegrass, perennial ryegrass, or a `Mixed`
  cool-season blend) in USDA zone 7a, southeast Pennsylvania. Warm-season grass types
  exist in the model but get no cool-season program (no fall feedings or seeding).
- **Soil and rain**: NOAA USCRN station PA Avondale (WBANNO 3761), read from a local
  data lake. Silver = hourly observations; gold = daily means and `gdd50`. The lake
  trails real time by about a day, so "today" is station-local and usually partial.
- **Air temperature / humidity now**: a Home Assistant patio sensor.
- **Forecast**: OpenWeatherMap 5-day / 3-hourly, bucketed by local date. Whole-day
  judgments skip the partial first and last day.
- All temperatures are °F. Timing windows use **5 cm** soil; the live reading, grub
  window and soil-temperature outlook use **10 cm**.

## Staleness rules

| Situation | What TurfOps does |
|-----------|-------------------|
| Newest soil reading older than 72 h | Not used as current; `soil_observed_at` shows the real time and the dashboard shows a banner |
| Station data older than 5 days | Timing windows trust only confirmed crossings and otherwise fall back to the calendar |
| Forecast fetch fails | A forecast up to 6 h old is kept |
| Lake read fails | The last good soil summary is kept; timing and disease recommendations drop out rather than guess |

## Thresholds (TTTF, zone 7a)

| Metric | Threshold | Meaning |
|--------|-----------|---------|
| Soil temp 10 cm | 60–75°F | Grub control window |
| Air temp | > 85°F | Fertilizer stress risk |
| Soil moisture (fraction) | < 0.10 | Irrigation needed |
| Soil moisture (fraction) | > 0.40 | Saturated — avoid fertilizer |
| Soil 5 cm, 5-day mean held 5 d | 45 / 50 / 55°F rising | Spring pre-emergent: early / ideal / closed (or 200 GDD) |
| Soil 5 cm, 5-day mean held 5 d | 70 / 65 / 55°F falling | Fall pre-emergent: opens / late / closed |
| Soil 5 cm, 5-day mean held 5 d | ≤ 75°F from Aug 15 → 55°F | Fall seeding opens → soil-side close |
| Median first fall freeze (air min ≤ 32°F) | −45 d / −30 d (KBG −60 / −45, PRG −35 / −21) | Fall seeding: end of ideal / last chance |
| GDD base 50°F | 50–150 | Spring nitrogen readiness; spring broadleaf window |
| GDD base 50°F | 500–700 | Grub control (egg-laying → peak hatch) |

GDD per day is `max((max + min) / 2 − 50, 0)`, summed from January 1.

## Seed vs pre-emergent

Seeding and pre-emergent never share a season. A logged overseed blocks that season's
pre-emergent, and a logged pre-emergent blocks that season's seeding. Fall pre-emergent
is the alternative to fall seeding, never a companion. Core aeration follows the fall
seeding window and goes quiet once seeding or a fall pre-emergent is logged. A fall
broadleaf herbicide is dropped if the lawn was just seeded.

## Nitrogen

Annual lawn targets, lbs N / 1000 sq ft (minimum – maximum, recommended):

| Grass | Range | Recommended |
|-------|-------|-------------|
| Tall fescue | 2.0 – 4.0 | 3.5 |
| Kentucky bluegrass | 3.0 – 5.0 | 4.0 |
| Perennial ryegrass | 2.0 – 4.0 | 3.0 |
| Fine fescue | 1.0 – 3.0 | 2.0 |
| Mixed (cool-season) | 2.0 – 4.0 | 3.0 |

A feeding's N is rate × N% for lawn applications only (plant fertilizer is excluded).
"Apply N" recommendations go on hold while fertilizer is blocked (saturated soil, heat)
or a Warning-level heat forecast is active, are capped to the remaining budget, and
become "target reached" at zero.

## Disease risk

Each disease has its own model over a daily weather series: observed days from the lake
(a usable day needs ≥ 12 hours plus both its pre-dawn 03–08 and afternoon 13–17 hours)
and the rest of today plus the outlook from the forecast. Scores are native to each
model; **only the tier is comparable across diseases.**

| Disease | Model | Moderate | High | Severe |
|---------|-------|----------|------|--------|
| Brown patch | Fidanza E-index | ≥ 0 | ≥ 5 | ≥ 6 |
| Dollar spot | Smith-Kerns probability | ≥ 10% | ≥ 20% (published action threshold) | ≥ 40% |
| Pythium blight | Nutter-criteria score (0–5) | ≥ 2 | ≥ 3 | ≥ 4 |
| Gray leaf spot | Suitability index (experimental) | ≥ 25 | ≥ 50 | ≥ 75 |
| Red thread | Suitability index (experimental) | ≥ 25 | ≥ 50 | ≥ 75 |

Lawn history raises only the headline tier: overseeding within 60 days raises gray leaf
spot, and no fertilizer within 60 days raises red thread.

**Actions.** Low: none. Moderate: monitor (with a heads-up when a High / Severe day is
forecast within 3 days). High / Severe: apply a preventative, unless a logged fungicide
rated Good or better on that disease is still inside its residual (21 days systemic,
14 days contact, 7 days shorter under Severe), which reads `Protected`. Strobilurins
protect against Pythium at High but not at Severe. Red thread's remedy is nitrogen, not a
spray. The curative program is always shown so the owner can decide from what they see;
TurfOps has no symptom input.

**Efficacy and rotation.** Fungicide options come from a static FRAC-class efficacy
matrix checked against University of Kentucky PPA-1 (2024): ratings 4 / 3.5 →
Excellent, 3 / 2.5 → Good, 2 / 1.5 → Fair. Ratings are per FRAC class; a class whose
actives disagree is left out for that disease rather than averaged. The suggested class
is the best non-restricted option that is not the class used last; a product on the
shelf only breaks ties between equally good options. Multi-site classes: mancozeb = M3,
chlorothalonil = M5. TurfOps never gives application rates; the label governs.

## Recommendations and the shelf

- A recommendation that calls for a product carries a *need* (category, FRAC classes,
  herbicide timing, targets, amendment kind). The shelf is matched strictly: a product
  without the asked-for fact recorded does not count. Status: OnHand, Low (only
  Low-stock products fit), NotOnHand, Partial (some needs covered).
- Stock is hand-set In stock / Low / Out; there are no quantities. `archived` means no
  longer used.
- Dismissing or addressing a recommendation is not permanent. It comes back when it
  escalates past the answered severity or when its episode ends: 7 days for weather
  alerts, 21 days for disease, 120 days for seasonal work.
- A product's stored label rate is only ever a starting point: "verify against your
  label".
