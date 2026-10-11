---
parent: high-level-design
prefix: RULE
---

# Agronomic Rules

## Context and Design Philosophy

The agronomic rules answer the seasonal and weather-driven questions that have no dedicated model elsewhere: when to feed, when grub preventative goes down, when a broadleaf herbicide works, when to water, when to hold an application for rain or heat, and how high to mow. Each rule is a pure function of three inputs (the environmental summary, the lawn profile and the application log) and returns at most one recommendation. This component also owns the nitrogen guard, which covers every recommendation its source marks as a feeding so that feeding advice never contradicts a fertilizer block and never exceeds the annual nitrogen budget.

The component also owns the two quantities the rules lean on and the dashboard shows directly: year-to-date growing degree days (base 50°F) with the crabgrass germination model, and the annual lawn nitrogen budget.

Principles, in the HLD's order of precedence:

- **One answer everywhere.** Anything that must agree with the seeding or pre-emergent window is not a rule. Pre-emergent, seeding, core aeration and the fall broadleaf herbicide's seeding caveat are owned by Timing windows (`backend/src/logic/timing/companions.rs`). Disease alerts are owned by Disease risk. The rule engine is constructed once (`backend/src/state.rs`) and called only by the Recommendation feed, so the dashboard, Recommendations page, shopping list and MCP all see the same rule output.
- **Withhold rather than invent.** A rule whose required input is `None` returns `None` (Rust `?`). It does not fall back to a calendar guess. GDD is the exception by design: it only *adds* timing or urgency, so a missing GDD value degrades to the calendar and soil-temperature logic.
- **Infer from the log.** "Already applied this season" is read from the application log. No rule has a toggle.

## Rule Contract and Engine

`Rule::evaluate(env, profile, history) -> Option<Recommendation>` (`backend/src/logic/rules/mod.rs`). `RulesEngine::evaluate` (`backend/src/logic/rules/engine.rs`) runs the ten rules in a fixed order, collects their `Some` results, then calls `nitrogen_guard::apply` with `chrono::Local::now().date_naive()`. The fixed order is spring nitrogen, broadleaf herbicide, grub control, fertilizer block, fall fertilization, rain delay, irrigation, heat stress, application window, mowing height. The output order is the feed's source order for rules.

| Input | Fields the rules read | Producer |
|---|---|---|
| `EnvironmentalSummary.soil_temp_7day_avg_f` | 7-day mean of station **10 cm** soil temperature | Weather data access (`datasources/weather.rs` `fetch_summary`) |
| `.soil_temp_trend` | Rising / Falling / Stable / Unknown. Compares the newest 24 hourly readings with the 24 before; Unknown under 48 readings | Weather data access |
| `.ambient_temp_7day_avg_f` | 7-day mean air temperature from the station readings | Weather data access |
| `.current` | live 10 cm soil temperature, `primary_soil_moisture()` (10 cm, else 5 cm, else 20 cm), ambient air from Home Assistant | Data sync (soil omitted when the newest observation is over 72 h old) |
| `.precipitation_7day_total_mm` | lake 7-day rain total | Weather data access |
| `.forecast` | OWM daily summaries (local dates, `hours_covered`) and 3-hourly points | Weather data access |
| `.gdd_base50_ytd` | sum of gold `gdd50` from Jan 1 | Data sync (`fetch_gdd_ytd`) |
| `LawnProfile` | `grass_type`, `lawn_size_sqft` | profile |
| `history` | the newest 1000 applications for the profile (`RULE_HISTORY_LIMIT`) | Recommendation feed |

"Today" is the lawn's local date, from the `TZ` zone that deployment sets to the lawn's timezone. The rules, the guard and `/gdd` read it as `chrono::Local`; the nitrogen budget endpoint's cut-off and data sync's YTD GDD input take it from the app's `Clock` (`backend/src/clock.rs`), resolved once from the same `TZ`. Every date gate and year boundary uses it. Forecast day helpers take that date as an argument (`next_days(today, n)`, `max_temp_next_days(today, n)`), so the heat-stress and irrigation rules count days from the lawn's today. `GrubControlRule::evaluate_on(today, …)` is the only rule with an injectable date; the others are tested only on the day the suite runs.

**Grass-type gate.** TurfOps serves cool-season lawns. `GrassType::is_cool_season()` (`models/lawn_profile.rs`) is true for tall fescue, Kentucky bluegrass, perennial ryegrass, fine fescue and `Mixed` (a cool-season blend). Warm-season lawns are unsupported, and what the rules do for one is not intent.

**Amounts in advice.** Feeding advice states agronomic amounts: lb N per 1,000 sq ft (`*_N_RATE_LBS_PER_KSQFT` in `thresholds.rs`), the matching lawn total in lb N when the profile records `lawn_size_sqft` (with no size recorded, the total is omitted rather than assumed), and a lb N/1,000 maximum in the May cutoff. These amounts are intended, and the nitrogen budget cannot be expressed without them. Advice never states a product's label rate, and never any rate for a pesticide: grub control names actives "at label rate", and broadleaf herbicide names actives with weather conditions only (`grub_control.rs:165`, `broadleaf_herbicide.rs:139-142,187-191`). The early-fall text lists suitable N-P-K analyses, not products or label rates. Irrigation actions give a water depth, which is also an agronomic amount.

All numeric thresholds live in `backend/src/logic/rules/thresholds.rs`. Plant watering (`logic/plant_maintenance/watering.rs`) and the timing-window seed text (`logic/timing/recommendations.rs`) import from it, so changing a shared constant changes those components too.

## Rule Catalogue

| Rule (file) | Recommendation id(s) | Category | Severities | Season gate | Required inputs | ProductNeed declared |
|---|---|---|---|---|---|---|
| Spring nitrogen (`spring_nitrogen.rs`) | `spring_n_wait`, `spring_n_almost`, `spring_n_ready`, `spring_n_too_early`, `spring_n_may_cutoff` | Fertilizer | Info / Advisory / Warning | Feb–May | soil 7-day avg | `spring_n_ready` only: Fertilizer |
| Broadleaf herbicide (`broadleaf_herbicide.rs`) | `broadleaf_spring`, `broadleaf_fall` | Herbicide | Advisory (spring), Warning (fall, before Timing windows reconciles it) | spring by GDD or March; fall Sep 20–Oct 31 | soil 7-day avg; spring also needs trend Rising | Herbicide · PostEmergent · Broadleaf |
| Grub control (`grub_control.rs`) | `grub_control_<year>`, `grub_control_late_<year>` | GrubControl | Info / Advisory / Warning | May 15–Jul 4, or from Apr 1 while GDD 500–<1000 | soil 7-day avg **and** live 10 cm soil | InsectControl · Grubs |
| Fertilizer block (`fertilizer.rs`) | `fertilizer_block` | Fertilizer | Warning / Critical | year-round | `current` reading | none |
| Fall fertilization (`fall_fertilization.rs`, text in `fall_fertilization/recs.rs`) | `fall_fert_early`, `fall_fert_mid`, `fall_fert_winterizer` | Fertilizer | Advisory / Warning | Sep 1–Nov 30 | soil 7-day avg | Fertilizer |
| Rain delay (`rain_delay.rs`) | `rain_delay` | ApplicationTiming | Advisory / Warning / Critical | year-round | forecast | none |
| Irrigation (`irrigation_forecast.rs`) | `irrigation_forecast` | Irrigation | Advisory / Warning / Critical | year-round | forecast **and** soil moisture | Surfactant, `optional` |
| Heat stress (`heat_stress.rs`) | `heat_stress_forecast` | HeatStress | Advisory / Warning / Critical | year-round | forecast | none |
| Application window (`application_window.rs`) | `application_window` | ApplicationTiming | Info | year-round | forecast | none |
| Mowing height (`mowing_height.rs`) | `mowing_height` | Mowing | Info / Advisory / Warning | year-round | ambient 7-day avg | none |

A rule declares what its action calls for with `Recommendation::with_need(ProductNeed)`. The feeding rules (`spring_n_ready`, `fall_fert_*`) declare a nitrogen-fertilizer need: a Fertilizer whose nitrogen is above 0 or unrecorded, so a potash-only product on the shelf does not show a feeding as on hand. Matching a need against the shelf, the inventory badge and the shopping list belong to Product inventory.

### Spring nitrogen

Cool-season only, February through May, and needs the 7-day soil average. "Fed this spring" means a spring *feeding*: a turf `Fertilizer` application dated from Feb 1 of the current year through today with N% unrecorded or above 0, the same definition fall fertilization uses. Plant fertilizer and potash-only products are not feedings. Today the rule counts any `Fertilizer` row since Feb 1 (`spring_nitrogen.rs:53-60`), a divergence (Deferred #5).

| Condition | Result |
|---|---|
| May, not fed this spring | `spring_n_may_cutoff`, Info: last chance, prefer skipping spring N |
| May, fed | none |
| soil < 50°F, fed | `spring_n_too_early`, Warning |
| soil < 50°F, not fed | `spring_n_wait`, Info |
| 50 ≤ soil < 55°F, not fed, GDD ≥ 50 | `spring_n_ready`, Advisory (GDD confirms active growth) |
| 50 ≤ soil < 55°F, not fed, GDD < 50 or `None` | `spring_n_almost`, Info |
| 55 ≤ soil ≤ 65°F, not fed | `spring_n_ready`, Advisory; GDD ≥ 150 adds a "well established" note, GDD ≥ 50 an "active growth" note |
| fed (50–65°F), or soil > 65°F | none |

Waiting texts point to the Timing page for pre-emergent; the rule gives no pre-emergent advice of its own.

### Broadleaf herbicide

Cool-season only. Silent while any `Overseed` is logged within the last 60 days (`HERBICIDE_OVERSEED_BUFFER_DAYS`). Already-treated is read from `PostEmergent` applications this year: month ≤ 4 blocks spring, month ≥ 9 blocks fall.

- **Spring** (`broadleaf_spring`, Advisory): (GDD 50–150 inclusive, or the month is March), soil 45–55°F inclusive, and `soil_temp_trend` is Rising. GDD adds a data point and a "winter annuals actively growing" note.
- **Fall** (`broadleaf_fall`, Warning): Sep 20–Oct 31 with soil 50–65°F. GDD is not used.

Timing windows' `reconcile_fall_herbicide` adjusts `broadleaf_fall` after the rules run, following the log. If the lawn was seeded this fall, the recommendation is dropped. If the fall seeding window is still usable and nothing is logged, it is capped at Advisory with an "only if NOT seeding" caveat. If a fall pre-emergent is logged or seeding is closed, it is left unchanged.

### Grub control

The calendar window is May 15–Jul 4. GDD widens it: from Apr 1 the window is open while YTD GDD is in [500, 1000). The 1000 upper bound closes the GDD extension, because YTD GDD only grows. Silent once any `GrubControl` or `Insecticide` application is logged on or after Apr 1 this year. Needs the 7-day soil average and the live 10 cm reading.

- Soil 60–75°F inclusive → `grub_control_<year>`. Severity is Warning when GDD ≥ 700 (peak egg-hatch) or ≤ 14 days remain to Jul 4, otherwise Advisory. Past Jul 4 (GDD still under 1000) the text says the usual close has passed. The action names actives "at label rate".
- Soil > 75°F → `grub_control_late_<year>`, Info, "apply soon".
- Soil < 60°F → none.

### Fertilizer block

Cool-season only, needs `current`. Two independent checks: ambient air > 85°F (Home Assistant; skipped when absent), and soil moisture < 0.10 (drought) or > 0.40 (saturated). Any hit emits `fertilizer_block`. The severity is Critical when air > 90°F or moisture < 0.05, otherwise Warning. Its presence puts the feeding recommendations on hold (see Nitrogen Guard).

### Fall fertilization

Cool-season only, Sep 1–Nov 30, needs the soil average. A *feeding* is a turf `Fertilizer` application (`is_turf()`) this year, dated today or earlier, with N% unrecorded or above 0. This excludes potash-only products and plant fertilizer. The spacing requirement is ≥ 21 days since the newest feeding of the year, so an August feeding still spaces September's.

| Phase | Fires when | Severity |
|---|---|---|
| September → `fall_fert_early` | no fall feeding yet, spacing met, soil 45–65°F | Warning |
| October → `fall_fert_mid` | fewer than 2 fall feedings, spacing met, soil 45–65°F | Warning if none yet, else Advisory |
| November → `fall_fert_winterizer` | fewer than 3 fall feedings, spacing met, soil ≥ 40°F | Warning if none yet, else Advisory |

### Weather rules

- **Rain delay.** Checks the forecast's 3-hourly points within 12 h, then 24 h, then 48 h. Within 12 h with probability ≥ 70% or ≥ 2.5 mm → Critical. Within 24 h with probability ≥ 50% or ≥ 2.5 mm → Warning. Within 48 h with probability ≥ 30% or ≥ 5 mm → Advisory. The first level that matches wins.
- **Irrigation.** Needs both the forecast and soil moisture. Silent at moisture ≥ 0.20, or when the 5-day forecast *amount* exceeds 2.5 mm (probability alone never cancels it). Severity is Critical below 0.10, Warning below 0.15, otherwise Advisory. It declares an optional wetting-agent need, which never puts the recommendation on the shopping list.
- **Heat stress.** Cool-season only. Uses the maximum daily high over today plus the next 3 local days. Severity is Advisory from 85°F, Warning from 90°F and Critical from 95°F. "Hot days" counts the consecutive days from the start, ignoring a leftover partial day that is cooler.
- **Application window.** Considers the first 5 daily summaries and judges only full days (`is_full_day`). A day is *good* when its mean temperature is ≥ 50°F with a high ≤ 85°F, the day before is dry (or, for the first summary, lake 7-day rain is under 25 mm) and the day itself and the next are dry. Dry means under 2.5 mm and under 50% probability. Wind and humidity add to the score but are not required. The highest-scoring good day is announced as Info. With no good day, there is no recommendation.
- **Mowing height.** Cool-season only. Uses the 7-day ambient average: above 85°F → Warning (heat-stress height), above 75°F → Advisory (summer height), at or above 50°F → Info (spring/fall height), below 50°F → none.

## Nitrogen Guard

`nitrogen_guard::apply` (`backend/src/logic/rules/nitrogen_guard.rs`) keeps feeding advice from contradicting a fertilizer block and from exceeding the annual nitrogen budget. It covers exactly the recommendations their source marks as a **feeding**: a recommendation that asks the owner to apply nitrogen to the lawn now. The guard keeps no list of ids. Each source marks a recommendation when it builds it, so a new feeding source is covered without changing the guard, and a source whose one id can carry feeding or non-feeding advice marks only the feeding form.

| Source | Marked as a feeding | Not marked |
|---|---|---|
| Agronomic rules | `fall_fert_early`, `fall_fert_mid`, `fall_fert_winterizer`, `spring_n_ready` | `spring_n_may_cutoff` (advises against feeding) and every other rule |
| Disease management | `disease_red_thread` while its advice is to feed (nitrogen-hungry: no fertilizer logged in 60 days) | `disease_red_thread` when its advice is "stay the course" |
| Landscape plants | none | Fertilizing tasks (plant fertilizer is outside the lawn budget) |
| Soil tests, only if soil-test advice rejoins the feed | `soil_test_npk` when it asks for N > 0 | `soil_test_npk` asking only for P or K |

The guard reads the whole feed to find the blockers, edits only marked recommendations, and runs after every source and before inventory enrichment and answered state. Placing the pass there belongs to Recommendation feed; this component owns what the pass does and the marks on the rules' own feedings.

For each feeding, in this order:

1. **Budget data point.** The guard adds "N budget remaining: *r* of *target* lb/1000 sqft". It is the only author of this point; sources do not add their own. Remaining = the grass type's `recommended_lbs_per_1000sqft` − lawn N applied in today's calendar year up to and including today.
2. **Target reached.** When remaining is under 0.05 lb, the nitrogen part of the advice is withdrawn: the title gains "— annual nitrogen target reached" and the description says to skip the nitrogen or use a product without it. When the advice is nitrogen only (every rule feeding, red thread), that is the whole action: severity drops to Info, `suggested_action` is cleared and `needs` is emptied, so the feeding never shops. A non-nitrogen part (P or K in soil-test advice) keeps its action, need and severity.
3. **Cap.** When remaining is under the largest single feeding any rule suggests (`LARGEST_FEEDING_LBS`), the nitrogen amount is limited: "keep this application at or under *remaining*" lb N. Severity and any non-nitrogen part are unchanged.
4. **Hold.** When `fertilizer_block` is present at any severity, or `heat_stress_forecast` is present at Warning or above, the whole recommendation is held, because no fertilizer of any kind should go down: severity becomes Info, the title is prefixed "On hold —" and the description explains the block. Hold and target-reached compose: both edits apply.

The guard runs before answered state, so a dismissed `fertilizer_block` still holds feedings: dismissing the alert does not change the conditions that rule out fertilizer. It runs before inventory enrichment, so a target-reached feeding has no nitrogen need left and never reaches the shopping list for nitrogen.

**Divergence.** The guard runs inside `RulesEngine::evaluate` (`engine.rs:48-54`) over rule output only, and recognizes feedings by the fixed id list `FEEDING_IDS` (`nitrogen_guard.rs:12-17`) rather than a mark set by each source (Deferred #1, #2). Its target-reached branch clears the whole action and every need (`nitrogen_guard.rs:64-74`), which matches intent for nitrogen-only feedings.

## Lawn Nitrogen and the Annual Budget

Lawn nitrogen per application is `Application::turf_nitrogen_lbs()`, which belongs to Applications: N% / 100 × rate per 1000 sq ft, only for turf applications with both values > 0, otherwise `None`. The guard and the budget endpoint both sum only this value. Applications missing an analysis or a rate do not count against the budget.

Annual targets (`models/nitrogen_budget.rs` `annual_n_target`, lb N / 1000 sq ft / year):

| Grass | Min | Max | Recommended (the target) |
|---|---|---|---|
| Tall fescue | 2.0 | 4.0 | 3.5 |
| Kentucky bluegrass | 3.0 | 5.0 | 4.0 |
| Perennial ryegrass | 2.0 | 4.0 | 3.0 |
| Fine fescue | 1.0 | 3.0 | 2.0 |
| Mixed (cool-season) | 2.0 | 4.0 | 3.0 |

`annual_n_target` also has rows for the warm-season grasses the profile accepts; they are not designed behavior.

## GDD Accumulation and the Crabgrass Model

GDD per day is the gold layer's precomputed `gdd50`, which uses the same formula as `max((max+min)/2 − 50, 0)`. The app only sums it.

- **For the rules:** data sync sets `gdd_base50_ytd` from `fetch_gdd_ytd(today)`, with `today` the lawn's local date from the app's `Clock`. That function sums gold rows from Jan 1 of `today`'s year through `today` and returns `None` when no rows exist. On a read error the value is left unset and a warning is logged. The rules and `/gdd` therefore agree on the year, including on the evening of Dec 31.
- **For the endpoint:** `logic/gdd.rs` `accumulate_daily_gdd` builds a running total that resets at each calendar year (rows must be sorted ascending). `build_gdd_summary` takes the last cumulative value as `current_gdd_total`. The Environmental history endpoint reuses `accumulate_daily_gdd` for its GDD series.
- Gold days with `gdd50` NULL are skipped, so a gap in the lake lowers the total without any signal.

`crabgrass_model(gdd)` (`logic/gdd.rs`) maps the YTD total to a `CrabgrassStatus`:

| YTD GDD | Status |
|---|---|
| < 75 | PreGermination |
| 75 – < 150 | ApproachingGermination |
| 150 – < 200 | GerminationLikely |
| ≥ 200 | PostGermination |

`germination_threshold` is 200. `estimated_germination_date` is always `null`. The model is informational only: no recommendation reads it. Spring pre-emergent advice comes only from Timing windows, which uses 200 GDD as one of its spring-close boundaries.

## API Endpoints

| Method | Path | Request | Response | Errors |
|---|---|---|---|---|
| GET | `/api/v1/gdd` (`api/gdd.rs`) | `?year=` (default: local current year) | `GddSummary { year, current_gdd_total, daily_history[DailyGdd], crabgrass_model, last_computed_date }`. The current year runs through today; other years run Jan 1–Dec 31 | 400 for a year chrono cannot represent; a lake read error is mapped by the shared error type (data-lake task failure → 503). With no lake client configured it returns a data-unavailable error, so the widget hides; today it returns an empty history and total 0 (Deferred #9) |
| GET | `/api/v1/nitrogen-budget` (`api/nitrogen_budget.rs`) | `?year=` (default: local current year) | `NitrogenBudget { year, target, applied, remaining (≥ 0), percent_of_target (≤ 999), applications[date, product_name, nitrogen_pct, rate_per_1000sqft, n_lbs_per_1000sqft] sorted by date, grass_type_target }` | 404 with no lawn profile; 400 for an unrepresentable year |

The budget counts the applications dated in the requested calendar year on or before today, the same set the guard counts, so the widget and the feed show the same "remaining". A future-dated entry counts once its date arrives: `budget_window(year, today)` (`api/nitrogen_budget.rs`) counts dates in `[Jan 1, min(Jan 1 of the next year, today + 1))`, with `today` from the app's `Clock`. The MCP server exposes both endpoints through its own tools.

The component has no tables of its own. It reads applications and the lawn profile, and caches nothing.

## UI

Both widgets render on the Dashboard (`frontend/src/pages/Dashboard.tsx`). The dashboard fetches each with `.catch(() => null)` and omits a widget whose fetch fails.

- **`GddWidget.tsx`**: shows the YTD total against a fixed "/ 200", a progress bar, a crabgrass status badge (`CRABGRASS_STATUS_LABELS` / `CRABGRASS_STATUS_COLORS` in `types/index.ts`, label always shown with the color), a sparkline of the last 60 days' cumulative GDD with a dashed 200 reference line (drawn only when there are more than 5 points), and the footnote "Crabgrass germinates at ~200 GDD".
- **`NitrogenBudgetWidget.tsx`**: shows applied vs. target, a bar colored green (≤ 80%), amber (> 80%) or red (> 100%), the grass type's min–max range, "Remaining" or "Over budget by", and one row per counted application. When nothing counted, it prompts the user to record N-P-K when logging.

## Degradation

| Missing input | Effect |
|---|---|
| Lake / soil average (`None`), including station data over 72 h old (data sync then withholds the 7-day figures and the trend) | Spring N, broadleaf, grub, fall fertilization silent; irrigation loses the lake rain total; spring broadleaf loses the trend |
| Live soil reading (stale > 72 h) | Grub silent even if the 7-day average exists; fertilizer block keeps only the air check; irrigation silent |
| Home Assistant air | Fertilizer block keeps the soil-moisture checks |
| Forecast | Rain delay, irrigation, heat stress, application window silent; the guard can no longer hold on heat |
| GDD (`None`) | Spring N stays on soil temperature only; broadleaf spring uses March only; grub uses the calendar window only with no peak-hatch escalation |
| Trend Unknown/Stable | Spring broadleaf silent |

## Decisions & Alternatives

| Decision | Chosen | Alternatives Considered | Rationale |
|---|---|---|---|
| Rule shape | Pure functions over summary, profile and log, at most one recommendation each | Rules that query their own data | Testable on fixed inputs and identical for every caller; a missing input degrades cleanly (HLD decision). |
| GDD in rules | Additive only: opens earlier, escalates, adds notes; `None` falls back to calendar + soil | GDD-gated rules | Lake outages or a short record must not silence seasonal advice. [inferred] |
| Grub window upper bound | Close the GDD extension at 1000 | Open-ended once GDD ≥ 500 | YTD GDD never falls, so without a bound the window would stay open until Dec 31 (`grub_control.rs` doc comment). [inferred] |
| Feeding vs. blocks | One guard pass over every recommendation marked as a feeding, after all sources and before inventory and answered state; a block or Warning+ heat stress holds the whole recommendation, the budget caps only its nitrogen; the guard alone writes "N budget remaining" | Each feeding rule checking weather itself; a guard over rule output only; the budget withdrawing the whole recommendation | Each source judges its own window and sees no other source. One pass over the whole feed guarantees "feed" never sits beside "avoid fertilizer" and no feeding exceeds the budget, whichever source produced it. Under a block or heat stress no fertilizer of any kind should go down, while the budget limits nitrogen only, so phosphorus or potassium the soil lacks still stands. One author for the budget point keeps one figure per recommendation. Running before inventory keeps a target-reached feeding off the shopping list; running before answered state keeps a dismissed block in force. |
| How the guard finds feedings | Each source marks a recommendation as a feeding when it builds it | A fixed list of feeding ids in the guard; inferring from the Fertilizer category or a Fertilizer need | The same id can carry feeding and non-feeding advice: `disease_red_thread` says "feed the lawn" while the lawn is nitrogen-hungry and "stay the course" once fed, and `soil_test_npk` may ask for P or K only. Only the source knows which form it built. A category or need test would also catch plant Fertilizing tasks and the May cutoff, and an id list misses every new source. |
| Amounts in feeding advice | State lb N/1,000 sq ft, the lawn total and the May maximum; never a label rate or a pesticide rate | No amounts at all; typical label rates | Nutrient amounts are agronomy and do not depend on the product, and the budget cannot be expressed without them. How much of a product to apply is set by its label (HLD non-goal). |
| Clock | The lawn's local date (`TZ` = the lawn's zone, read as `chrono::Local` or the app's `Clock`) for every date gate and year boundary | The UTC date | Window days, the feeding season and the budget year are lawn-local calendar days; one configured clock gives every component the same day (HLD decision). |
| N rules and the budget | Feedings are capped to the remaining annual budget and become "target reached" at zero | A fixed fall program regardless of budget | The fixed fall program overshoots a low-N grass's target; recorded as the owner-approved default in project memory. |
| Budget accounting | Only `turf_nitrogen_lbs()` dated on or before today; target = the grass type's recommended value | Counting plant fertilizer; estimating unrecorded analyses; counting future-dated entries | Plant fertilizer is not on the lawn; an unrecorded analysis cannot be counted without inventing a number; a future-dated entry has not been applied, and the endpoint and the guard must show the same remaining figure. |
| What counts as a feeding in the log | A turf Fertilizer application, dated on or before today, with N% unrecorded or above 0, for spring and fall alike | Any Fertilizer row | Plant fertilizer and potash-only products put no nitrogen on the lawn; one definition keeps spring and fall consistent. |
| Feeding product need | A Fertilizer with nitrogen above 0 or unrecorded | Any Fertilizer | A 0-0-x product on the shelf cannot deliver the feeding, so it must not show the feeding as on hand. |
| Lawn total without a lawn size | Omitted; the per-1,000 sq ft amount stands | Assume 5,000 sq ft | Withhold rather than invent: a total for an unknown size is a made-up number. |
| `/gdd` with no weather lake | Data-unavailable error | Empty history, total 0 | A zero total reads as real data (PreGermination) and invents a crabgrass status. |
| Lawn type | Cool-season lawns only; `GrassType::Mixed` is a cool-season blend and gets every rule | A warm-season program | TurfOps serves one cool-season lawn (HLD non-goal: warm-season lawns are unsupported). |
| Fall broadleaf vs. seeding | The rule emits; Timing windows reconciles against the log | A user toggle; the rule reading the seeding window | Owner decision: fall broadleaf follows the log; seeding logic stays in one component. |
| Pre-emergent, seeding, aeration | Not rules; owned by Timing windows | 10 cm soil-temperature rules | One answer everywhere: they must agree with the 5 cm windows (HLD tenet). |
| Crabgrass model | Informational status on the dashboard, no recommendation | Driving pre-emergent advice | Pre-emergent advice has one source (Timing windows). [inferred] |
| Plan status for rule-covered activities | The rules expose each covered activity's current-season state and the seasonal plan reads it | The plan's own 10 cm triggers | One answer everywhere: the plan must not say Active while the feed is silent, or the reverse; timing activities already work this way (RULE-ENG-010, PLAN-ACT-021). |
| Rain delay and feedings | Rain delay is not a guard blocker; a feeding can stand beside a rain-delay alert | Holding feedings while `rain_delay` is active | Rain delay times an application within the next two days, while a feeding window spans weeks; together they read as "feed, after the rain", not as a contradiction. A fertilizer block or heat stress rules out fertilizer for days, so only those hold. |
| Irrigation cancel test | Only forecast *amount* cancels the alert | Probability ≥ 50% cancels | A 50% chance of a trace does not relieve dry soil (code comment and test). [inferred] |

## Open Questions & Future Decisions

### Resolved
1. ✅ TurfOps serves cool-season lawns; `Mixed` is a cool-season blend and gets every rule. Warm-season lawns are unsupported.
2. ✅ A fertilizer block or a Warning+ heat forecast holds a feeding whole; the annual budget caps only its nitrogen. The guard covers every recommendation marked as a feeding, before inventory and answered state, and is the only author of "N budget remaining".
3. ✅ Fall broadleaf herbicide follows the log through Timing windows.
4. ✅ Feeding advice states nitrogen amounts (lb N/1,000 sq ft, the lawn total, the May maximum); grub and broadleaf advice defer to the label. Agronomic amounts are intended; label rates and pesticide rates are not given.
5. ✅ A dismissed `fertilizer_block` still holds feedings, because the guard runs before answered state.
6. ✅ Feedings are the recommendations their source marks as asking for lawn nitrogen now (Nitrogen Guard table). Landscape-plant Fertilizing tasks, `spring_n_may_cutoff` and red thread's "stay the course" are not feedings and are not held or capped.

### Deferred
1. **Guard placement and feeding recognition (divergence).** The guard runs inside `RulesEngine::evaluate` over rule output only (`engine.rs:48-54`) and matches the fixed id list `FEEDING_IDS` (`nitrogen_guard.rs:12-17`) instead of reading a feeding mark set by each source. Feedings from other sources escape it, and `soil_test_npk`, while soil-test advice remains in the feed, adds its own "N budget remaining" point (`api/recommendation_feed.rs:172-178`). Moving the pass is Recommendation feed's change; the mark, the rules marking their feedings and the guard reading the mark are this component's.
2. **Red thread feeding is not guarded** (divergence). While the lawn is nitrogen-hungry, `disease_red_thread` asks the owner to feed, but it carries no feeding mark and is not in `FEEDING_IDS` (`nitrogen_guard.rs:12-17`), so a fertilizer block, heat stress or an exhausted budget does not hold or cap it. Disease management marks it while its advice is to feed; fixed together with #1.
3. The guard's cap edits only `description`; `suggested_action` still states the rule's full nitrogen amount, so the text asks for both (`nitrogen_guard.rs:75-81`, `fall_fertilization/recs.rs:57-63`, `spring_nitrogen.rs:333-338`).
4. **Spring "fed" counts any `Fertilizer` row** (divergence, RULE-SPRING-010). Plant fertilizer and potash-only products trigger `spring_n_too_early` or silence the rule (`spring_nitrogen.rs:53-60,73-77`); fall fertilization already filters with `is_turf()` and N% > 0. A correctly timed feeding followed by a cold snap still triggers `spring_n_too_early`, which the definition does not change.
5. **The feeding need is plain `Fertilizer`** (divergence, RULE-SPRING-008, RULE-FALL-007), so a 0-0-x product on the shelf satisfies it and the recommendation shows On hand (`spring_nitrogen.rs:328-331`, `fall_fertilization/recs.rs:187-189`). Needs a nitrogen requirement on `ProductNeed`, which Product inventory matches.
6. **The lawn total assumes 5,000 sq ft** (divergence, RULE-ENG-009). With no lawn size on the profile, feeding text falls back to `DEFAULT_LAWN_SIZE_SQFT` (`thresholds.rs:184`) and states a total the data does not support.
7. Grub ids carry the year (`grub_control_<year>`, `grub_control.rs:122,177`), while the other rule ids rely on answered-state expiry. Grub and broadleaf "already applied" checks ignore `is_turf()`.
8. Gold days with `gdd50` NULL are dropped (`datasources/weather.rs:217`), so a gap silently lowers YTD GDD and the crabgrass status. With no lake client, `/gdd` returns 200 with total 0 and PreGermination instead of a data-unavailable error (`api/gdd.rs:36-43`; divergence, RULE-GDD-007).
9. Spring broadleaf is silent whenever the trend is Unknown (under 48 readings) or Stable (`broadleaf_herbicide.rs:71`). Fall broadleaf checks no trend, though its doc comment says "falling" (`broadleaf_herbicide.rs:14-15,80-86`). The application window's doc comment lists wind and humidity as requirements, but they only add to the score.
10. Application window: equal scores pick the *last* good day (`max_by_key`), and a final summary with no next day is assumed dry (`application_window.rs:52-55,168-175`).
11. Irrigation can say "no significant rain … for 0 days": `dry_days` stops at the first day with ≥ 2.5 mm or ≥ 50% probability, while the cancel test uses only the 5-day amount (`irrigation_forecast.rs:59-66,86-90`).
12. `GddWidget` hard-codes 200 instead of using `crabgrass_model.germination_threshold` (`GddWidget.tsx:41,48,68`).
13. Tests for spring nitrogen, broadleaf and the GDD branch of grub read the real clock and pass vacuously out of season. Only grub has `evaluate_on`.
14. CLAUDE.md understates two rules: irrigation fires below 0.20 moisture (Advisory), not only below 0.10; and spring N has no "GDD 50–150" range: ≥ 50 promotes 50–55°F soil to ready, ≥ 150 only adds a note.
15. **Size limits.** `spring_nitrogen.rs` is 441 lines and `grub_control.rs` is 347 lines, both over the 300-line limit. Functions over 50 lines: `GrubControlRule::evaluate_on` (~180), `FertilizerRule::evaluate` (~105), `FallFertilizationRule::evaluate` (~88), `SpringNitrogenRule::evaluate` (~83), `build_ready_to_fertilize` (~74), `HeatStressRule::build_recommendation` (~70), `BroadleafHerbicideRule::evaluate` (~69), `MowingHeightRule::evaluate` (~66), and the rain-delay, irrigation, application-window, fall-text builders and `nitrogen_guard::apply` (51–65 each).

## References

- `backend/src/logic/rules/` (engine, thresholds, ten rules, `nitrogen_guard.rs`)
- `backend/src/logic/gdd.rs`, `backend/src/models/gdd.rs`, `backend/src/api/gdd.rs`
- `backend/src/models/nitrogen_budget.rs`, `backend/src/api/nitrogen_budget.rs`
- `frontend/src/components/GddWidget.tsx`, `frontend/src/components/NitrogenBudgetWidget.tsx`
- Boundaries: `logic/timing/companions.rs` (Timing windows), `api/recommendation_feed.rs` (Recommendation feed), `models/application.rs` `turf_nitrogen_lbs` (Applications), `models/inventory.rs` `ProductNeed` (Product inventory)
- `docs/agronomy-methods.md` (Thresholds, Nitrogen); Missouri Extension g6705; K-State Extension (fall feeding)
