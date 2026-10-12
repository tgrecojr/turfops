# Agronomic Rules — EARS Specs

Design: [agronomic-rules-design.md](agronomic-rules-design.md). Prefix `RULE`. Facets: `ENG` (rule contract and engine), `SPRING` (spring nitrogen), `BROAD` (broadleaf herbicide), `GRUB` (grub control), `BLOCK` (fertilizer block), `FALL` (fall fertilization), `WX` (rain delay, irrigation, heat stress, application window, mowing height), `GUARD` (nitrogen guard), `BUDGET` (annual nitrogen budget), `GDD` (growing degree days and the crabgrass model), `UI` (dashboard widgets).

"Soil average" below is the environmental summary's 7-day mean of station 10 cm soil temperature. "Live soil" is the current 10 cm reading, absent when the newest station observation is over 72 h old.

## Rule contract and engine

- [x] **RULE-ENG-001**: Each agronomic rule shall be a pure function of the environmental summary, the lawn profile and the application log, performing no I/O and returning at most one recommendation.
- [x] **RULE-ENG-002**: The rules engine shall evaluate the ten rules in this order and return their recommendations in the same order: spring nitrogen, broadleaf herbicide, grub control, fertilizer block, fall fertilization, rain delay, irrigation, heat stress, application window, mowing height.
- [x] **RULE-ENG-003**: If an input a rule requires is missing, then that rule shall return no recommendation instead of falling back to a calendar date.
- [x] **RULE-ENG-004**: Where year-to-date GDD is missing, the spring nitrogen, broadleaf herbicide and grub control rules shall still evaluate on soil temperature and the calendar; GDD shall only add timing, urgency or notes.
- [x] **RULE-ENG-005**: The rules shall judge every date gate and year boundary on the `today`, and every forecast hour window on the `now`, of the one clock reading the caller passes (the feed's `Moment` from the app clock), and shall read no clock themselves.
- [x] **RULE-ENG-006**: The rules shall treat a `Mixed` grass type as a cool-season lawn and give it every cool-season rule.
- [x] **RULE-ENG-007**: Rule advice text shall not state a product's label rate or any rate for a pesticide; grub control and broadleaf herbicide advice shall name active ingredients and defer to the label.
- [x] **RULE-ENG-008**: When a rule gives feeding advice, it shall state the nitrogen amount in lb N per 1,000 sq ft and, when the lawn profile records a lawn size, the matching total for the lawn.
- [x] **RULE-ENG-009**: If the lawn profile records no lawn size, then feeding advice shall omit the lawn total rather than assume a size.
- [x] **RULE-ENG-010**: The rules shall expose, for each seasonal-plan activity they also cover (`spring_nitrogen`; `early_fall_fert`, `mid_fall_fert` and `winterizer`; `spring_herbicide` and `fall_herbicide`; `grub_preventative`), its current-season state, judged on today's local date, the environmental summary, the application log and the assembled recommendation feed (after the nitrogen guard and the timing companions, before the owner's answers are applied), in this precedence: Done (the rule's own log check finds the work) → Due (the activity's recommendation is in the feed) or Not needed (that recommendation is a feeding the nitrogen guard marked target reached) → Passed (the rule's window is over) → Unknown (an input the rule requires is missing while its window could be open) → Not yet.
- [x] **RULE-ENG-011**: For the `spring_nitrogen` plan activity, the rules shall report Done when a lawn feeding (`Application::is_feeding()`) is dated from February 1 of the current year through today; Due when the feed carries `spring_n_ready`; Passed from May 1 on, or when the soil average is above 65°F; Unknown from February 1 through April 30 when the soil average is missing; otherwise Not yet.
- [x] **RULE-ENG-012**: For the `early_fall_fert`, `mid_fall_fert` and `winterizer` plan activities (fall phases 1, 2 and 3: September, October, November), the rules shall report Done when the number of lawn feedings dated from September 1 of the current year through today is at least the phase number; Due when the feed carries that phase's recommendation (`fall_fert_early`, `fall_fert_mid`, `fall_fert_winterizer`); Passed after the phase's month ends; Unknown during the phase's month when the soil average is missing; otherwise Not yet.
- [x] **RULE-ENG-013**: For the `spring_herbicide` plan activity, the rules shall report Done when the broadleaf herbicide rule's spring check finds a PostEmergent application dated January through April of the current year; Due when the feed carries `broadleaf_spring`; Passed from May 1 on, or in April when year-to-date GDD is above 150 or missing (the spring window can no longer open); Unknown in March or April when the soil average is missing; otherwise Not yet.
- [x] **RULE-ENG-014**: For the `fall_herbicide` plan activity, the rules shall report Done when the broadleaf herbicide rule's fall check finds a PostEmergent application dated September through December of the current year; Due when the feed carries `broadleaf_fall`; Passed after October 31; Unknown from September 20 through October 31 when the soil average is missing; otherwise Not yet.
- [x] **RULE-ENG-015**: For the `grub_preventative` plan activity, the rules shall report Done when the grub control rule's check finds a GrubControl or Insecticide application dated from April 1 of the current year; Due when the feed carries `grub_control_<year>` or `grub_control_late_<year>`; Passed after July 4 when the GDD-extended window is not open; Unknown while the grub window is open and the soil average or live soil is missing; otherwise Not yet.

## Spring nitrogen

- [x] **RULE-SPRING-001**: The spring nitrogen rule shall evaluate only from February through May, and only when the soil average is present.
- [x] **RULE-SPRING-002**: When the spring nitrogen rule evaluates in May and no spring feeding is logged, it shall emit `spring_n_may_cutoff` at Info, advising that skipping spring nitrogen is preferable.
- [x] **RULE-SPRING-003**: When the spring nitrogen rule evaluates from February through April with the soil average below 50°F and a spring feeding logged, it shall emit `spring_n_too_early` at Warning.
- [x] **RULE-SPRING-004**: When the spring nitrogen rule evaluates from February through April with the soil average below 50°F and no spring feeding logged, it shall emit `spring_n_wait` at Info.
- [x] **RULE-SPRING-005**: When the spring nitrogen rule evaluates from February through April with the soil average from 50°F to below 55°F, no spring feeding logged, and year-to-date GDD of at least 50, it shall emit `spring_n_ready` at Advisory.
- [x] **RULE-SPRING-006**: When the spring nitrogen rule evaluates from February through April with the soil average from 50°F to below 55°F, no spring feeding logged, and year-to-date GDD below 50 or missing, it shall emit `spring_n_almost` at Info.
- [x] **RULE-SPRING-007**: When the spring nitrogen rule evaluates from February through April with the soil average from 55°F to 65°F inclusive and no spring feeding logged, it shall emit `spring_n_ready` at Advisory, adding a "well established" note at year-to-date GDD ≥ 150 or an "active growth" note at GDD ≥ 50.
- [x] **RULE-SPRING-008**: When the spring nitrogen rule emits `spring_n_ready`, it shall declare a nitrogen-fertilizer product need (a Fertilizer whose nitrogen is above 0 or unrecorded), so a potash-only product does not satisfy it.
- [x] **RULE-SPRING-009**: When the spring nitrogen rule tells the owner to wait, it shall point to the Timing page for pre-emergent and give no pre-emergent advice of its own.
- [x] **RULE-SPRING-010**: The spring nitrogen rule shall count as a spring feeding only a feeding as defined by Applications (`Application::is_feeding()`: a turf Fertilizer application with nitrogen unrecorded or above 0) dated from Feb 1 of this year through today.

## Broadleaf herbicide

- [x] **RULE-BROAD-001**: While any Overseed application is logged within the last 60 days, the broadleaf herbicide rule shall emit nothing.
- [x] **RULE-BROAD-002**: When year-to-date GDD is between 50 and 150 inclusive or the month is March, the soil average is 45–55°F inclusive, the soil trend is Rising, and no PostEmergent application is logged this year in January through April, the broadleaf herbicide rule shall emit `broadleaf_spring` at Advisory.
- [x] **RULE-BROAD-003**: When the date is Sep 20–Oct 31, the soil average is 50–65°F inclusive, and no PostEmergent application is logged this year from September on, the broadleaf herbicide rule shall emit `broadleaf_fall` at Warning (the timing-window pass later reconciles it against seeding).
- [x] **RULE-BROAD-004**: When the broadleaf herbicide rule emits a recommendation, it shall declare a Herbicide · PostEmergent · Broadleaf product need.

## Grub control

- [x] **RULE-GRUB-001**: The grub control rule shall treat its window as open from May 15 to Jul 4, and also from Apr 1 while year-to-date GDD is at least 500 and below 1000.
- [x] **RULE-GRUB-002**: While a GrubControl or Insecticide application is logged on or after Apr 1 of this year, the grub control rule shall emit nothing.
- [x] **RULE-GRUB-003**: The grub control rule shall evaluate only when both the soil average and live soil are present.
- [x] **RULE-GRUB-004**: When the grub control window is open and the soil average is 60–75°F inclusive, the grub control rule shall emit `grub_control_<year>`, at Warning when year-to-date GDD is at least 700 or 14 or fewer days remain to Jul 4, otherwise at Advisory.
- [x] **RULE-GRUB-005**: When the grub control window is open and the soil average is above 75°F, the grub control rule shall emit `grub_control_late_<year>` at Info.
- [x] **RULE-GRUB-006**: When the grub control rule emits a recommendation, it shall declare an InsectControl · Grubs product need.

## Fertilizer block

- [x] **RULE-BLOCK-001**: When the current ambient air temperature is above 85°F, or the current soil moisture is below 0.10 or above 0.40, the fertilizer block rule shall emit `fertilizer_block`, at Critical when the air is above 90°F or the moisture is below 0.05, otherwise at Warning.
- [x] **RULE-BLOCK-002**: If the current ambient air temperature is missing, then the fertilizer block rule shall apply only the soil-moisture checks; if live soil moisture is missing, it shall apply only the air check.

## Fall fertilization

- [x] **RULE-FALL-001**: The fall fertilization rule shall evaluate only from Sep 1 to Nov 30, and only when the soil average is present.
- [x] **RULE-FALL-002**: The fall fertilization rule shall count only a feeding as defined by Applications (`Application::is_feeding()`: a turf Fertilizer application with nitrogen unrecorded or above 0) dated this year on or before today.
- [x] **RULE-FALL-003**: The fall fertilization rule shall emit nothing until at least 21 days have passed since the most recent feeding of the year.
- [x] **RULE-FALL-004**: When it is September, no fall feeding is logged, spacing is met and the soil average is 45–65°F inclusive, the fall fertilization rule shall emit `fall_fert_early` at Warning.
- [x] **RULE-FALL-005**: When it is October, fewer than two fall feedings are logged, spacing is met and the soil average is 45–65°F inclusive, the fall fertilization rule shall emit `fall_fert_mid`, at Warning if no fall feeding is logged, otherwise at Advisory.
- [x] **RULE-FALL-006**: When it is November, fewer than three fall feedings are logged, spacing is met and the soil average is at least 40°F, the fall fertilization rule shall emit `fall_fert_winterizer`, at Warning if no fall feeding is logged, otherwise at Advisory.
- [x] **RULE-FALL-007**: When the fall fertilization rule emits a recommendation, it shall declare a nitrogen-fertilizer product need (a Fertilizer whose nitrogen is above 0 or unrecorded), so a potash-only product does not satisfy it.

## Weather rules

- [x] **RULE-WX-001**: The rain delay rule shall judge the forecast points within 12, then 24, then 48 hours by their highest precipitation probability and their total amount, skipping a window whose total is under 0.1 mm and whose highest probability is under 50%, and shall emit `rain_delay` at Critical when the 12-hour window reaches 70% or 2.5 mm; otherwise at Warning when the 24-hour window reaches 50% or 2.5 mm; otherwise at Advisory when the 48-hour window reaches 30% or 5 mm.
- [x] **RULE-WX-002**: When both the forecast and soil moisture are present, soil moisture is below 0.20, and the 5-day forecast precipitation amount is at most 2.5 mm, the irrigation rule shall emit `irrigation_forecast`, at Critical below 0.10, Warning below 0.15, otherwise Advisory.
- [x] **RULE-WX-003**: The irrigation rule shall let only the forecast precipitation amount, never its probability alone, cancel the alert.
- [x] **RULE-WX-004**: When the irrigation rule emits a recommendation, it shall declare an optional Surfactant need, which never puts the recommendation on the shopping list.
- [x] **RULE-WX-005**: When the highest daily high over today and the next three local days reaches 85°F, the heat stress rule shall emit `heat_stress_forecast`, at Advisory from 85°F, Warning from 90°F and Critical from 95°F.
- [x] **RULE-WX-006**: When at least one of the first five daily forecast summaries is a good full day (mean ≥ 50°F, high ≤ 85°F, under 2.5 mm and under 50% precipitation on that day and the day after, and the same on the day before — or, for the first summary, under 25 mm of rain in the lake's 7-day total), the application window rule shall announce the highest-scoring good day as `application_window` at Info; with no good day it shall emit nothing.
- [x] **RULE-WX-007**: The application window rule shall judge only full forecast days, never a partial first or last day.
- [x] **RULE-WX-008**: When the 7-day ambient average is above 85°F the mowing height rule shall emit `mowing_height` at Warning (heat-stress height); above 75°F at Advisory (summer height); at or above 50°F at Info (spring/fall height); below 50°F it shall emit nothing.

## Nitrogen guard

- [x] **RULE-GUARD-001**: The nitrogen guard shall act on exactly the recommendations their source marked as a feeding (advice to apply nitrogen to the lawn now), and shall keep no list of recommendation ids.
- [x] **RULE-GUARD-002**: When the fall fertilization or spring nitrogen rule emits `fall_fert_early`, `fall_fert_mid`, `fall_fert_winterizer` or `spring_n_ready`, it shall mark the recommendation as a feeding; no other rule recommendation shall carry the mark.
- [x] **RULE-GUARD-003**: For each feeding, the nitrogen guard shall add the data point "N budget remaining: *remaining* of *target* lb/1000 sqft", where target is the grass type's recommended annual nitrogen and remaining is target minus lawn nitrogen applied in the current calendar year on or before today, shown no lower than 0.
- [x] **RULE-GUARD-004**: When the remaining annual nitrogen is under 0.05 lb N/1000 sq ft and a feeding's advice is nitrogen only, the nitrogen guard shall lower it to Info, append "— annual nitrogen target reached" to the title, replace the description with the applied-versus-target figures and "skip this feeding, or use a product without nitrogen", clear the suggested action, and remove its product needs.
- [D] **RULE-GUARD-005**: When the remaining annual nitrogen is under 0.05 lb N/1000 sq ft and a feeding's advice also covers a non-nitrogen nutrient, the nitrogen guard shall withdraw only the nitrogen part and keep the remaining action, product need and severity.
- [x] **RULE-GUARD-006**: When the remaining annual nitrogen is at least 0.05 lb but less than the largest single feeding any rule suggests, the nitrogen guard shall limit the feeding to the remaining amount, telling the owner to keep the application at or under it, without changing its severity.
- [x] **RULE-GUARD-007**: When the nitrogen guard limits a feeding to the remaining amount, no part of the recommendation (description, suggested action or data points) shall state a nitrogen amount above the remaining amount.
- [x] **RULE-GUARD-008**: While the feed contains `fertilizer_block` at any severity or `heat_stress_forecast` at Warning or above, the nitrogen guard shall hold every feeding: lower it to Info, prefix its title with "On hold —", and explain in the description that current conditions rule out fertilizer.
- [x] **RULE-GUARD-009**: When a feeding is both on hold and at target reached, the nitrogen guard shall apply both edits.
- [x] **RULE-GUARD-010**: The nitrogen guard shall treat `fertilizer_block` and `heat_stress_forecast` as blockers whether or not the owner has dismissed or addressed them.
- [x] **RULE-GUARD-011**: When the nitrogen guard turns a feeding into "target reached" (RULE-GUARD-004), it shall also mark the feeding itself as target reached, so readers of the feed (RULE-ENG-010) can tell without parsing its text.

## Annual nitrogen budget

- [x] **RULE-BUDGET-001**: The nitrogen budget and the nitrogen guard shall count only lawn nitrogen as defined by `Application::turf_nitrogen_lbs()` (turf applications with both an N percentage and a rate), so plant fertilizer and applications missing either value count nothing.
- [x] **RULE-BUDGET-002**: The system shall use these recommended annual nitrogen targets in lb N/1000 sq ft: tall fescue 3.5, Kentucky bluegrass 4.0, perennial ryegrass 3.0, fine fescue 2.0, Mixed 3.0.
- [x] **RULE-BUDGET-003**: When `GET /api/v1/nitrogen-budget` is called, the system shall return the target, applied, remaining (never below 0), percent of target (capped at 999), the grass type's min–max range, and each counted application sorted by date, for the requested calendar year (default: the server's local current year).
- [x] **RULE-BUDGET-005**: The nitrogen budget endpoint and the nitrogen guard shall both count only applications dated on or before the server's local today, so a future-dated entry counts against neither.
- [x] **RULE-BUDGET-004**: If `GET /api/v1/nitrogen-budget` is called with no lawn profile, then the system shall return 404; if called with a year chrono cannot represent, then 400.

## Growing degree days and the crabgrass model

- [x] **RULE-GDD-001**: The system shall take each day's GDD (base 50°F) from the weather lake's gold `gdd50` value and only sum it, never recompute it.
- [x] **RULE-GDD-002**: When supplying year-to-date GDD to the rules, the system shall sum gold `gdd50` from Jan 1 of the server's local current year through the server's local today.
- [x] **RULE-GDD-003**: When accumulating a GDD history, the system shall reset the running total at each calendar year.
- [x] **RULE-GDD-004**: The crabgrass model shall report PreGermination below 75 GDD, ApproachingGermination from 75 to below 150, GerminationLikely from 150 to below 200, and PostGermination at 200 or more, with a germination threshold of 200.
- [x] **RULE-GDD-005**: The crabgrass model shall be informational only; no recommendation shall read it.
- [x] **RULE-GDD-006**: When `GET /api/v1/gdd` is called, the system shall return the year's running GDD history, its current total and the crabgrass model for the requested year (default: the server's local current year), through today for the current year and Jan 1–Dec 31 for other years; if the year cannot be represented, then it shall return 400.
- [x] **RULE-GDD-007**: If no weather lake is configured, then `GET /api/v1/gdd` shall return a data-unavailable error instead of a zero total and a PreGermination status.

## Dashboard widgets

- [x] **RULE-UI-001**: The GDD widget shall show the year-to-date total against the 200 GDD germination threshold with a progress bar, a crabgrass status badge showing its label with its color, a sparkline of the last 60 days' running total with a dashed reference line at 200 when there are more than five points, and the footnote "Crabgrass germinates at ~200 GDD".
- [x] **RULE-UI-002**: The nitrogen budget widget shall show applied versus target with a bar colored green at or below 80%, amber above 80% and red above 100%, the grass type's min–max range, "Remaining" or "Over budget by", and one row per counted application; when nothing counted, it shall prompt the owner to record N-P-K when logging.
