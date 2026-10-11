# Recommendation Feed & Answered State — EARS Specs

Design: [recommendation-feed-design.md](recommendation-feed-design.md). Prefix `FEED`. Facets: `REC` (the recommendation contract), `PIPE` (building the feed), `STATE` (answered-state policy), `API` (endpoints), `UI` (Recommendations page and dashboard layout).

## Recommendation contract

- [x] **FEED-REC-001**: The system shall represent every piece of advice it shows on any surface as a `Recommendation` with an id, category, severity, title, description, explanation, data points, optional suggested action, product needs, optional inventory status, creation time, and `dismissed` / `addressed` flags.
- [x] **FEED-REC-002**: The system shall order recommendation severities Info < Advisory < Warning < Critical, and every severity comparison in the feed shall use that order.
- [x] **FEED-REC-003**: When serializing a recommendation, the system shall always include `needs` (an empty list when the advice calls for no product) and shall omit `inventory` when no inventory status is set.
- [ ] **FEED-REC-004**: Wherever the web UI shows a recommendation's severity (Recommendations page cards, the detail pane, dashboard alert cards, the shopping list), it shall show the severity's symbol and label in ink text on a tint of its status color (Info ℹ, Advisory →, Warning ⚠, Critical !), never the color alone and never white text on a fill.

## Building the feed

- [x] **FEED-PIPE-001**: The system shall build the recommendation feed in one shared function that the dashboard, the Recommendations page, the shopping list and the MCP recommendation tools all read, and none of those surfaces shall filter, re-score, re-word or reorder the feed's recommendations (a surface may only truncate the feed).
- [x] **FEED-PIPE-002**: When building the feed, the system shall gather recommendations from its sources in this order: agronomic rules, disease risk, timing windows, landscape-plant tasks, application follow-ups.
- [x] **FEED-PIPE-003**: When building the feed, the system shall run the timing-window source after the agronomic rules, so that the timing pass can edit or drop the rule-produced `broadleaf_fall` recommendation.
- [x] **FEED-PIPE-004**: When building the feed, the system shall run the nitrogen guard exactly once, after every source has contributed, over every recommendation its source marked as a feeding (advice to apply nitrogen to the lawn now: today the rules' `fall_fert_*` and `spring_n_ready`, and `disease_red_thread` while its advice is to feed), whatever its source and without a fixed list of ids.
- [x] **FEED-PIPE-005**: When building the feed, the system shall not treat landscape-plant Fertilizing tasks or `spring_n_may_cutoff` as feedings for the nitrogen guard.
- [x] **FEED-PIPE-006**: The system shall add an "N budget remaining" data point to a feed recommendation only through the nitrogen guard, so a guarded feeding carries exactly one such point and no source adds its own.
- [x] **FEED-PIPE-007**: When building the feed, the system shall run the inventory pass after every source and the nitrogen guard, and before applying answered state.
- [x] **FEED-PIPE-008**: When applying answered state, the system shall compare each answer's stored severity against the recommendation's severity after the nitrogen guard has run.
- [x] **FEED-PIPE-009**: As the last stage of building the feed, after answered state is applied, the system shall sort the feed by descending severity with a stable sort, so recommendations of equal severity keep source order.
- [D] **FEED-PIPE-010**: The system shall emit at most one recommendation per id in the feed.
- [x] **FEED-PIPE-011**: When building the feed, the system shall read the application log once (newest first, up to 1000 rows) and give that same log to every source.
- [x] **FEED-PIPE-012**: When building the feed, the system shall use the server's local date (`Local::now()`, governed by `TZ`) as "today" for landscape-plant tasks and application follow-ups.
- [x] **FEED-PIPE-014**: When running the inventory pass for the feed, the system shall use every non-archived shelf product as the shelf.
- [x] **FEED-PIPE-015**: If the disease-risk source fails while building the feed (weather lake or forecast unavailable), then the system shall omit disease recommendations, log a warning, and return the rest of the feed.
- [x] **FEED-PIPE-016**: If the timing-window source fails while building the feed (weather lake unavailable), then the system shall omit timing-window and aeration recommendations and leave `broadleaf_fall` as the agronomic rules produced it.
- [x] **FEED-PIPE-017**: If no lawn profile exists, then the recommendations and dashboard endpoints shall return 404 without building the feed.
- [x] **FEED-PIPE-018**: The system shall not add soil-test advice (pH, N-P-K or micronutrient recommendations) to the recommendation feed; the Soil Tests page and the MCP `soil_test_advice` tool are its only surfaces.

## Answered state

- [x] **FEED-STATE-001**: The system shall persist at most one answered-state row per recommendation id, holding `dismissed`, `addressed`, the severity the recommendation showed when answered (nullable), and the time of the last answer.
- [x] **FEED-STATE-002**: While an answered-state row has `dismissed` or `addressed` set, the system shall suppress the recommendation with that id only if the recommendation's current severity is not above the stored severity and the recommendation's episode has not ended.
- [x] **FEED-STATE-003**: When a recommendation's current severity is above the severity stored with its answer, the system shall show the recommendation as active.
- [x] **FEED-STATE-004**: When a recommendation's current severity is below the severity stored with its answer, the system shall keep suppressing it until its episode ends.
- [x] **FEED-STATE-005**: If an answered-state row has no stored severity, then the system shall suppress the recommendation until its episode ends and shall never treat it as escalated.
- [x] **FEED-STATE-006**: The system shall assign a recommendation's answer episode by the first matching rule: an id starting `application_followup_` never expires; the id `fertilizer_block` lasts 7 days; categories Irrigation, HeatStress, FrostWarning, Mowing and ApplicationTiming last 7 days; a landscape-plant Watering task lasts 7 days (FEED-STATE-010); category DiseasePressure lasts 21 days; every other recommendation lasts 120 days.
- [x] **FEED-STATE-007**: The system shall measure an answer's episode from the time of the last answer recorded for that id.
- [x] **FEED-STATE-008**: The system shall apply the same suppression policy to Dismissed and Mark Addressed answers; they differ only in the label shown.
- [x] **FEED-STATE-009**: When an answer suppresses a recommendation, the system shall copy the row's `dismissed` and `addressed` flags onto the recommendation; when it does not, the system shall leave the recommendation active and leave the row unchanged.
- [ ] **FEED-STATE-010**: The system shall give a landscape-plant Watering task (a "during dry spells" reminder) the 7-day weather episode, not the 120-day seasonal one.

## Endpoints

- [x] **FEED-API-001**: When `GET /api/v1/recommendations` is called without `include_inactive=true`, the system shall return the feed with answered recommendations removed; with `include_inactive=true` it shall return every recommendation with answered ones flagged; both in the feed's order.
- [x] **FEED-API-002**: When `PATCH /api/v1/recommendations/:id` leaves `dismissed` or `addressed` true (after filling omitted flags from the stored row), the system shall upsert the row for that id and reset its last-answer time to now.
- [x] **FEED-API-003**: When `PATCH /api/v1/recommendations/:id` leaves both `dismissed` and `addressed` false (after filling omitted flags from the stored row), the system shall delete the row for that id (Restore).
- [x] **FEED-API-004**: When a `PATCH /api/v1/recommendations/:id` body omits `dismissed`, `addressed` or `severity`, the system shall keep that field's stored value.
- [x] **FEED-API-005**: When `PATCH /api/v1/recommendations/:id` succeeds, the system shall respond with `{id, dismissed, addressed}` as stored.
- [x] **FEED-API-006**: When `GET /api/v1/dashboard` is called, the system shall include the first three recommendations of the active feed, in the feed's order.

## Recommendations page and dashboard

- [x] **FEED-UI-001**: When the Recommendations page loads, it shall fetch the feed once with answered recommendations included and split it into active and answered lists in the browser.
- [x] **FEED-UI-002**: For each active recommendation, the Recommendations page shall show a card with the severity badge, the category, an inventory badge when the recommendation has an inventory status, the title, the description, and Mark Addressed and Dismiss buttons.
- [x] **FEED-UI-003**: When the owner clicks Mark Addressed or Dismiss on the Recommendations page, the page shall send a PATCH setting that flag with the recommendation's current severity, and shall mark the card answered without re-fetching the feed.
- [x] **FEED-UI-004**: When the owner selects a recommendation on the Recommendations page, the page shall show a detail pane with the title, severity, category, description, explanation, a data-points table, the suggested action, and an Inventory section listing each matching product (with its stock when not In stock) and each missing need with a link to `/inventory`.
- [x] **FEED-UI-005**: The Recommendations page shall offer a "Show dismissed / addressed (n)" toggle that lists every answered recommendation with a Restore button and explains that each one returns on its own if it becomes more severe or its episode ends.
- [x] **FEED-UI-006**: When the owner clicks Restore on an answered recommendation, the Recommendations page shall send a PATCH setting both `dismissed` and `addressed` to false.
- [x] **FEED-UI-007**: While the feed has no active recommendations, the Recommendations page shall show "No active recommendations. Your lawn is looking good!"
- [x] **FEED-UI-008**: The dashboard's Active Alerts shall show the dashboard endpoint's top three recommendations as read-only alert cards (severity badge, category, inventory badge, title, description) with no answer buttons.
- [ ] **FEED-UI-009**: The dashboard shall lay out, in order: the header (profile name, grass type, zone, last-updated time), the stale-soil banner when the summary's `soil_stale` flag is set (SYNC-MERGE-006), the connection indicators, the four gauges, the widget grid, and Active Alerts beside Recent Applications (five rows).
- [x] **FEED-UI-010**: While the dashboard tab is visible, the dashboard shall re-fetch every 30 seconds; while it is hidden it shall not poll; when it becomes visible again it shall re-fetch immediately.
- [x] **FEED-UI-011**: If a dashboard widget's own fetch fails on first load, then the dashboard shall hide that widget and render the rest of the page.
- [ ] **FEED-UI-013**: If a dashboard widget's re-poll fails after it has shown data, then the dashboard shall keep that widget's last good data, marked stale, instead of hiding it.
- [x] **FEED-UI-012**: The Recommendations page shall list active recommendations in the order the feed returns them, without reordering.
