---
parent: high-level-design
prefix: SOIL
---

# Soil Tests

## Context and Design Philosophy

A lab soil test is the only direct measurement TurfOps has of what the lawn's soil holds chemically: pH, the Mehlich-3 extractable macro- and micronutrients, organic matter and CEC. Weather tells the app *when* to act; the soil test tells it *what the soil lacks*. This component records those tests and turns the **latest** one into three kinds of advice: a pH correction, an N-P-K ratio fitted to the phosphorus and potassium levels and the remaining annual nitrogen budget, and a list of low nutrients. The advice is shown on the Soil Tests page and returned by the MCP `soil_test_advice` tool. It does not contribute recommendations to the shared recommendation feed (Feed Integration).

Guiding principles:

- **The latest test speaks for the lawn.** Advice is derived from one test only, the one with the newest `test_date` (the most recently entered one when several share that date). Older tests are history; they are listed and editable but never blended into advice. The page shows the test's age and, past 3 years, suggests retesting, so an old test never reads as current.
- **Pure derivation, recomputed on read.** Advice is a pure function of (latest test, lawn profile, this year's applications) in `backend/src/logic/soil_test_recommendations.rs`. Nothing derived is persisted; editing or deleting a test changes the advice on the next read.
- **Off the feed while testing is occasional.** The owner tests soil rarely, so one test, possibly years old, would otherwise drive standing alerts, shopping-list items and agent advice. The page and `soil_test_advice` present the test's reading on request; the feed carries none of it. Soil-test advice can rejoin the feed once testing is regular, under the rules in Feed Integration.
- **Advice names materials, not brands.** Each piece of advice names a material (lime or sulfur, a fertilizer analysis, a supplement, gypsum, dolomitic lime), never a product.
- **Absent measurements are not deficiencies, and not adequacy either.** A nutrient the lab did not report produces no deficiency line. Unreported phosphorus or potassium is "Not tested": no P₂O₅ or K₂O is advised for it, and the advice says it was not tested.
- **Agronomic amounts, never label rates.** A soil-test correction cannot be expressed without an amount, so advice states the agronomic amount per 1,000 sq ft: pounds of lime or elemental sulfur to move pH, and pounds of N, P₂O₅ and K₂O. These are amounts of a nutrient or a soil-correcting material, independent of any shelf product (HLD Non-Goals, "Label rates"). Advice never states a product's label rate. Pounds of an *example analysis* fertilizer (`product_rate_lbs_per_1000sqft`) and amounts of a fertilizer or amendment material in the nutrient suggestions are agronomic amounts too, except for a material that is also a pesticide active (copper sulfate, ferrous sulfate): that suggestion gives only the nutrient amount.

## Data Model and Persistence

Owned table: `soil_tests` (`backend/src/db/migrations/20240101000007_soil_tests.sql`).

| Column | Type | Notes |
|---|---|---|
| `id` | `BIGSERIAL` PK | |
| `lawn_profile_id` | `BIGINT NOT NULL` → `lawn_profiles(id) ON DELETE CASCADE` | Always the default (first) profile |
| `test_date` | `DATE NOT NULL` | Defines "latest" |
| `lab_name` | `TEXT` | Optional |
| `ph` | `DOUBLE PRECISION NOT NULL` | The only required measurement |
| `buffer_ph` | `DOUBLE PRECISION` | Stored and shown; not used by advice (Lime amount, below) |
| `phosphorus_ppm`, `potassium_ppm` | `DOUBLE PRECISION` | Drive the N-P-K advice |
| `calcium_ppm`, `magnesium_ppm`, `sulfur_ppm` | `DOUBLE PRECISION` | Secondary nutrients, evaluated with the micronutrients |
| `iron_ppm`, `manganese_ppm`, `zinc_ppm`, `boron_ppm`, `copper_ppm` | `DOUBLE PRECISION` | Micronutrients |
| `organic_matter_pct`, `cec` | `DOUBLE PRECISION` | Stored and shown; not used by advice |
| `notes` | `TEXT` | |
| `created_at` | `TIMESTAMPTZ DEFAULT NOW()` | Never updated |

Index `idx_soil_tests_profile_date (lawn_profile_id, test_date DESC)` serves both the list and the latest-test lookup.

The Rust model is `SoilTest` in `backend/src/models/soil_test.rs`; queries live in `backend/src/db/soil_test_queries.rs` (a private `SoilTestRow` maps rows to the model).

**Which test is "latest".** The latest test is the one with the newest `test_date`. When several share that date, the most recently entered one (newest `created_at`, then highest `id`) is latest, and the history list orders same-date tests the same way, so its first row is always the test the advice reads (SOIL-DATA-005). A second same-day entry is usually a corrected re-entry or a later lab report, so it reflects the owner's latest knowledge. Both queries take this order from one definition (`newest_first!` in `soil_test_queries.rs`: `test_date DESC, created_at DESC, id DESC`); it is a macro because sqlx accepts only `'static` SQL.

## API Endpoints

All handlers are in `backend/src/api/soil_tests.rs`, routed in `backend/src/main.rs`. Every handler first resolves the default lawn profile (`queries::get_default_lawn_profile`, the lowest `id`) except DELETE.

| Method | Path | Request | Response | Errors |
|---|---|---|---|---|
| GET | `/api/v1/soil-tests` | `?limit=` (default 50, clamped 1–200), `?offset=` (≥0) | `SoilTest[]`, newest `test_date` first | 404 no profile |
| POST | `/api/v1/soil-tests` | `CreateSoilTestRequest` (`test_date` `YYYY-MM-DD`, `ph` required, all else optional) | 201 + created `SoilTest` | 400 bad date; 400 `pH must be between 0 and 14`; 400 negative measured value; 404 no profile |
| PUT | `/api/v1/soil-tests/{id}` | same body (full replace of every user field) | 200 + updated row as stored | 400 as POST; 404 `Soil test {id} not found` |
| DELETE | `/api/v1/soil-tests/{id}` | — | 204 | 404 `Soil test {id} not found` |
| GET | `/api/v1/soil-tests/recommendations` | — | `SoilTestSummary` for the latest test | 404 `No soil tests found`; 404 no profile |

Validation covers the date format, `0 ≤ pH ≤ 14`, and non-negative measured values: buffer pH, every ppm value, organic matter % and CEC are concentrations or indices that cannot be negative, so a negative one is a typo and is rejected with 400 and a plain message naming the field (SOIL-API-011). The message reads "{Field} can't be negative. Check the value on your lab report." POST and PUT share one check (`validate_request`). Deleting an id that does not exist is 404, like PUT (SOIL-API-012). PUT and DELETE are keyed by `id` alone (the profile is not part of the `WHERE`), which is sound only under the single-lawn assumption.

`SoilTestSummary` (`models/soil_test.rs`) carries the test itself, `ph_recommendation: PhRecommendation | null`, `npk_recommendation: NpkRecommendation | null` and `micronutrient_recommendations[]`. The frontend mirrors these in `frontend/src/types/index.ts`.

**Boundary — nitrogen budget.** The recommendations endpoint loads the applications dated from Jan 1 through today on the lawn's calendar (the app `Clock`, via `queries::get_applications_for_profile_in_range`), and the evaluator counts only those dates (SOIL-NPK-004) — the basis of the nitrogen budget and the nitrogen guard, so a planned feeding is not yet spent. It sums `Application::turf_nitrogen_lbs()` (owned by Applications) against `annual_n_target(grass_type).recommended_lbs_per_1000sqft` (`backend/src/models/nitrogen_budget.rs`). Plant applications contribute nothing because `turf_nitrogen_lbs` excludes them.

## Advice Derivation

Entry point `generate_soil_test_recommendations(test, profile, apps)` runs three independent evaluators. Thresholds are constants in `backend/src/logic/soil_test_thresholds.rs`.

### pH correction — `calculate_ph_recommendation`

| Input | Rule |
|---|---|
| Target pH | 6.5 for cool-season lawns (`grass_type.is_cool_season()`, includes `Mixed`); warm-season lawns are unsupported |
| Dead band | No recommendation when `|target − pH| < 0.2` |
| pH below target | Amendment "Lime (calcitic or dolomitic)" |
| pH above target | Amendment "Elemental sulfur" |
| Soil type | From the profile; when unset, Loam is assumed and the explanation says so |

The agronomic amount (lb of lime or elemental sulfur per 1,000 sq ft) is a per-pH-unit factor by soil type (`lime_rate_per_ph_unit` / `sulfur_rate_per_ph_unit`, increasing with clay content) times the pH gap, capped at a per-treatment maximum (`soil_test_thresholds.rs`). The explanation tells the owner to split larger needs over applications months apart.

**Lime amount and buffer pH** (SOIL-PH-007, deferred). Water pH says how acidic the soil is; buffer pH measures its reserve acidity, which is what governs how much lime it takes to move pH, and the soil-type factor is only a proxy for it. A buffer-pH lime amount needs a calibration for the buffer the lab used — SMP, Mehlich and Sikora buffers read differently — and a soil test does not record which buffer that was. Until it does, the soil-type factor sets every lime amount and a reported `buffer_ph` is stored and shown only.

### N-P-K ratio — `calculate_npk_recommendation`

| Nutrient | Low | Adequate | High |
|---|---|---|---|
| Phosphorus (Mehlich-3 ppm) | < 15 | 15–50 | > 50 |
| Potassium (Mehlich-3 ppm) | < 100 | 100–175 | > 175 |
| Not reported | **Not tested**: no P₂O₅ / K₂O advised for it | | |

Per-nutrient intent, in lb of nutrient per 1,000 sq ft: P₂O₅ is supplied only when P is Low; K₂O is supplied when K is Low (more) or Adequate (maintenance), never when High or Not tested; N is supplied from the remaining annual budget, capped per application and zeroed when the remainder falls below a small floor. When none of N, P or K is wanted the function still returns `Some` with ratio `"N/A"`, all amounts zero and an explanation that no fertilizer is recommended. Otherwise `suggest_ratio` normalizes the three amounts to the smallest non-zero one to produce a ratio string (e.g. `3-1-2`) and maps the N/P/K presence pattern to one hard-coded "common commercial" analysis (seven fixed analyses such as a straight-N, a P-only, a K-only and several complete blends). `product_rate_lbs_per_1000sqft` back-calculates the pounds of that example analysis from the first advised nutrient it carries — N, else P₂O₅, else K₂O — rounded once to 0.1 lb so the page and the explanation show the same figure, so P- or K-only advice states pounds of its 0-20-0 or 0-0-50 example, never 0 (SOIL-NPK-010; an agronomic amount, Resolved #6). The explanation names the reported P/K levels and ppm, says which of P and K was not tested, the remaining budget, the advised amounts (non-zero ones only), a budget-cap note when little remains, and the Loam-default note. When nothing is advised it says so ("No fertilizer is needed now.") with the budget, never claiming an untested nutrient is adequate.

**Unreported P or K is "Not tested"** (SOIL-NPK-012, SOIL-NPK-013). Advising maintenance potash for a soil whose potassium was never measured, or describing an unmeasured nutrient as adequate, would be an invented answer. A test without P/K still gets nitrogen advice from the budget; it gets no P₂O₅ or K₂O, and its explanation names the untested nutrients instead of an empty list of levels. `classify_phosphorus` / `classify_potassium` map an unreported value to `NutrientLevel::NotTested`.

The evaluator itself has no seasonal or weather gate: it does not look at the date, the grass's feeding calendar, or the active `fertilizer_block` / heat-stress state. On the page and through MCP the advice is what the test calls for, not a call to act today; weather and budget holds belong to the nitrogen guard in the feed and would apply only if soil-test advice rejoins it (Feed Integration).

### Nutrient deficiencies — `evaluate_micronutrients`

Calcium, magnesium and sulfur are secondary nutrients, not micronutrients; they are evaluated in the same list, which the page titles "Secondary & micronutrients" (the API field keeps the name `micronutrient_recommendations`). Each reported nutrient below its threshold yields one `MicronutrientRecommendation` with `level: Low` and a fixed suggestion string. A nutrient at or above its threshold, or not reported, yields nothing. `High` is never produced here.

| Nutrient | Low below (ppm) | Suggested material (amount text omitted here) |
|---|---|---|
| Calcium | 500 | Gypsum (calcium sulfate) |
| Magnesium | 50 | Dolomitic lime (also raises pH) or Epsom salt |
| Sulfur | 10 | Elemental sulfur, or ammonium sulfate fertilizer |
| Iron | 5 | 0.4–0.8 oz of iron as a foliar application; the label sets the product amount |
| Manganese | 5 | Manganese sulfate |
| Zinc | 1.0 | Zinc sulfate |
| Boron | 0.5 | Borax, with a toxicity caution |
| Copper | 0.5 | 0.25–0.5 oz of copper; the label sets the product amount |

Every suggestion contains a fixed amount per 1,000 sq ft, not computed from the test (`evaluate_micronutrients`). Gypsum and elemental sulfur sit beside lime as soil-correcting materials; Epsom salt, manganese sulfate, zinc sulfate and borax are fertilizer or amendment materials, so an amount of each is an agronomic amount. Copper sulfate and ferrous sulfate are also registered pesticide actives (ferrous sulfate for moss control), so under the HLD's label-rate rule the copper and iron suggestions give only the nutrient amount, never an amount of the salt (SOIL-NUTR-004, SOIL-NUTR-005). The iron and copper amounts are those of the earlier ferrous sulfate (2–4 oz, about 20 % Fe) and copper sulfate (1–2 oz, about 25 % Cu) suggestions, restated as the nutrient. Ferrous sulfate is not a chelated iron, and the iron suggestion never calls it one.

## Feed Integration

**Intent: soil-test advice is not in the recommendation feed.** The Soil Tests page and the MCP `soil_test_advice` tool are its only surfaces. The feed (dashboard alerts, the Recommendations page, the shopping list, the MCP `recommendations` and `lawn_snapshot` tools) carries no soil-test recommendation, so no soil-test product need reaches the inventory pass or the shopping list. The reason is procedural, not technical: the owner tests soil too rarely for a single, possibly old test to drive standing advice.

No feed code reads soil tests: `recommendation_feed::all` has no soil-test source and the feed module has no soil-test adapter or need builders. If soil-test advice rejoins the feed, its adapter and need builders belong in soil-tests code beside the evaluator, not in the feed module.

**Rejoining the feed.** Soil-test advice can return to the feed once the owner tests on a regular schedule, so the latest test is reasonably current. It then enters as a feed source like the others, ahead of the nitrogen guard, inventory enrichment and answered state, with these ids, severities and needs:

| Id | Severity | Fires when | Declared `ProductNeed` |
|---|---|---|---|
| `soil_test_ph` | **Warning** when `|pH − target| > 1.0`, else **Advisory** | pH recommendation present | Lime: `SoilAmendment`, kinds `CalciticLime` \| `DolomiticLime`. Sulfur: `SoilAmendment`, kind `Sulfur` |
| `soil_test_npk` | **Advisory** | Any of the N/P/K amounts > 0 | `Fertilizer` (`"{ratio} fertilizer"`), no NPK constraint |
| `soil_test_micro_{nutrient}` | **Info** | Each Low nutrient | Calcium → `SoilAmendment` `Gypsum`; Magnesium → either-or (below; Deferred #1); Sulfur → `SoilAmendment` `Sulfur`; Iron / Manganese / Zinc / Boron / Copper → `Supplement` with that `ProductTarget` |

All are category `SoilTest` (the answered-state policy's 120-day seasonal bucket). Each carries data points and an `action` that states an amount. These rules apply:

- `soil_test_npk` is marked as a feeding only when it asks for nitrogen (N > 0). A P/K-only recommendation is not a feeding.
- Under a fertilizer block or Warning+ heat stress the guard holds the whole recommendation: no fertilizer of any kind goes down.
- When the annual nitrogen budget is reached or limits the amount, the guard caps only the nitrogen part; P₂O₅ and K₂O advice stands, with its need.
- The guard is the only author of the "N budget remaining" data point; the soil-test recommendation does not add its own.
- The magnesium need is either-or: a Supplement targeting Magnesium or a SoilAmendment of kind dolomitic lime satisfies it, matched by Product inventory (SOIL-REJOIN-007). Dolomitic lime supplies magnesium while raising pH; a magnesium supplement such as Epsom salt supplies it without moving pH. Either answers "do I have something for low magnesium".

The guard's behavior is owned by Agronomic rules and its placement by Recommendation feed; this component owns marking `soil_test_npk`.

## Degradation and Empty States

| Situation | Endpoint | Page |
|---|---|---|
| No lawn profile | 404 on every GET/POST/PUT | Error banner pointing to Settings to set up the lawn profile |
| No soil tests | `/recommendations` → 404 `No soil tests found` | Recommendations panel hidden (page skips the call); "No soil tests recorded yet" in history |
| DB error reading latest test | 500 | Error banner with the server-error sentence (try again; details in the server log) |
| pH within dead band, nothing low, K High, N budget spent | Summary with `ph: null`, NPK "N/A", empty list | "No fertilizer needed" with the explanation (see UI) |
| Profile soil type unset | Loam assumed, noted in text | same |

Every failed load, save or delete on the page sets the banner to `describeError(e)` (Web UI's human-readable errors): the server's message as written for a 400 (a bad date, an out-of-range pH), the Settings pointer for a missing profile, "try again" on a timeout; never a status code or response body.

The feed carries no soil-test advice, so none of these situations affects it.

## Soil Tests Page

`frontend/src/pages/SoilTests.tsx` (form and history), with the advice in `frontend/src/components/soilTests/` (`AdvicePanel`, `LevelBadge`, and `display.ts` for level words, test age, the retest rule, the nothing-to-apply test and the delete question), route `/soil-tests` (`frontend/src/App.tsx`), nav label "Soil Tests". API calls in `frontend/src/api/client.ts` (`getSoilTests`, `createSoilTest`, `updateSoilTest`, `deleteSoilTest`, `getSoilTestRecommendations`).

```
┌───────────────────────────────────────────────┐
│ Soil Tests                    [+ Add Soil Test]│
├───────────────────────────────────────────────┤
│ (form, when open) Core · Macronutrients (ppm) │
│   · Micronutrients (ppm) · Other   [Save]     │
├───────────────────────────────────────────────┤
│ Recommendations · test of <date> (<age>)      │
│ (retest suggestion when over 3 years old)     │
│ ┌ pH Adjustment ┐ ┌ Fertilizer Recommendation┐│
│ └───────────────┘ └──────────────────────────┘│
│ Secondary & micronutrients (table)            │
├───────────────────────────────────────────────┤
│ Test History: Date·Lab·pH·P·K·Ca·Mg·OM%  Edit │
│                                        Delete │
└───────────────────────────────────────────────┘
```

- **Load.** Lists tests; fetches the summary only when at least one test exists, else clears it.
- **Form.** One form for add and edit; pH is required client-side (`"pH is required"`), the date defaults to today in browser-local time (`todayLocalISO`), empty numeric fields are sent as absent. After save the form resets and the page reloads both lists.
- **Recommendations panel.** Headed by "Based on your test of Mar 23, 2026 (6 months old)": the date of the test the advice is based on and its age in whole months and years from `test_date` to today's local date (`GET /api/v1/today`; without it, the date alone). When that test is more than 3 years old the panel suggests retesting: extension guidance for turf is to test every 2–3 years, and beyond that the advice describes a soil that may have changed (SOIL-UI-012, SOIL-UI-014, SOIL-UI-015). pH card (current, target, amendment, amount, explanation); Fertilizer card (ratio, N/P₂O₅/K₂O amounts with P and K `LevelBadge`s, example analysis with amount, N budget remaining, explanation); deficiency table titled "Secondary & micronutrients" (nutrient, current, threshold, level, suggestion; SOIL-UI-018). `LevelBadge` renders the level word (Low, Adequate, High, Not tested) in a status color (`NUTRIENT_LEVEL_COLORS`), never color alone.
- **Nothing to apply.** When the N-P-K recommendation advises no N, P₂O₅ or K₂O, the Fertilizer card gives way to "No fertilizer needed" with the explanation and the remaining nitrogen, never a ratio of "N/A" and zero amounts (SOIL-UI-016). The backend always returns an `npk_recommendation`, so this, not an empty panel, is how a fully adequate soil reads.
- **History table** shows a subset of fields (no buffer pH, S, or micronutrients). Delete asks for confirmation naming the test's date ("Delete the soil test of Mar 23, 2026? This can't be undone, and it may be the test your advice is based on.") before deleting (SOIL-UI-017); a deleted test cannot be recovered and may be the one driving the advice.
- The page shows no shelf status: soil-test advice is not in the feed, so the inventory pass never sees it.

## MCP Exposure

The MCP server (MCP) exposes `soil_tests` (limit default 5, clamped 1–50) and `soil_test_advice` (the recommendations endpoint, untrimmed) in `backend/src/mcp/tools/log.rs`. Both return what the REST handlers return, amounts included. The MCP doctrine (`backend/src/mcp/instructions.rs:23-24`, "Never give an application rate except a product's stored `label_rate`") is worded more strictly than the HLD's amount rule and reads as forbidding the lime, sulfur and nutrient amounts this tool returns; the doctrine text belongs to MCP server (Deferred #8).

## Decisions & Alternatives

| Decision | Chosen | Alternatives Considered | Rationale |
|---|---|---|---|
| Which test drives advice | Newest `test_date` only; among tests sharing that date, the most recently entered | Blend or trend several tests; newest `created_at` overall; an arbitrary pick on a same-date tie | Labs report a point-in-time state; the newest sample supersedes older ones, and the test date (not entry date) is when it was true. On a same-date tie the later entry is usually a correction or a later report, so it carries the owner's latest knowledge; an arbitrary pick would let advice flip between reads. |
| Persistence of advice | Recompute on every read | Store advice with the test | Advice depends on the profile and this year's log, which change independently of the test. [inferred] |
| pH target | 6.5 for cool-season lawns (incl. Mixed), ±0.2 dead band | Per-species targets; a target range | Matches common extension guidance for cool-season turf; the dead band avoids advising on lab noise. [inferred] |
| Nutrient interpretation basis | Fixed Mehlich-3 ppm thresholds | Lab-supplied interpretation; per-lab extractant | One extractant keeps the thresholds simple; Mehlich-3 is the common mid-Atlantic extractant. [inferred] |
| Missing P / K | "Not tested": no P₂O₅ / K₂O for it, and the advice says it was not tested; N still advised from the budget | Treat as Adequate (maintenance K₂O); skip the whole N-P-K advice | Treating an unmeasured nutrient as adequate invents a reading and advises potash nobody showed a need for. Skipping the whole advice would lose the nitrogen side, which rests on the budget, not the test. |
| N in soil-test fertilizer advice | Tied to the remaining annual N budget, counting lawn feedings from Jan 1 through today on the lawn's calendar | Independent N recommendation | Keeps soil-test advice consistent with the nitrogen budget the rules use. [inferred] |
| Soil-test advice in the feed | Not in the feed: the Soil Tests page and MCP `soil_test_advice` are its only surfaces; it can rejoin once testing is regular | A feed source like the others; in the feed only while the latest test is recent | The owner tests soil rarely. A single, possibly old test should not drive standing alerts, shopping-list items or agent advice; the page and the tool still show what the test calls for whenever the owner looks. A recency cutoff would still let one test speak for a year of alerts. |
| Nitrogen guard over `soil_test_npk` (when in the feed) | Marked as a feeding only when it asks for N > 0; held whole under a fertilizer block or Warning+ heat stress; at the budget only its nitrogen is capped and P₂O₅ / K₂O stand; the guard alone writes "N budget remaining" | Exempt from the guard; the evaluator re-implementing block and heat checks; dropping the whole recommendation at the budget | Under a block or heat stress no fertilizer should go down, whatever its analysis. The budget limits nitrogen only, so phosphorus and potassium the soil lacks are still worth applying. One author for the budget point keeps one figure per recommendation. |
| Feed ids (when in the feed) | Fixed ids (`soil_test_ph`, `soil_test_npk`, `soil_test_micro_<x>`) | Ids including the test id | The answered-state policy expires answers by episode, so ids need not encode the test. [inferred] |
| Feed severities (when in the feed) | pH Warning beyond 1.0 units else Advisory; N-P-K Advisory; deficiencies Info | Uniform severity | A large pH gap limits nutrient availability broadly; single-nutrient lows are informational. [inferred] |
| Product needs (when in the feed) | Lime/sulfur kinds for pH; any Fertilizer for N-P-K; gypsum for Ca; per-nutrient Supplement for micros | Encode NPK ratios in the need; a second Supplement need when only P or K is short (as planned in `docs/plans/product-inventory.md`) | Shelf facts carry NPK, but matching on ratio would rarely find a product; category-level need answers "do I have a fertilizer". The planned P/K Supplement need is not implemented. [inferred] |
| Lime amount basis | The soil-type factor; a reported buffer pH is stored and shown only | Buffer pH with one calibration (e.g. SMP only); a buffer-type field with a calibration per buffer | Buffer pH measures the reserve acidity that sets lime requirement, but its calibration depends on the lab's buffer, which a test does not record; one fixed calibration would give wrong lime amounts for labs using another buffer. The owner's lab reports no buffer pH. A buffer-type field is the way in (Deferred). |
| Stale tests | Show the test's age; suggest retesting past 3 years; advice still shown | Hide advice past an age; no age shown | Extension guidance for turf is a test every 2–3 years. Hiding the only test the owner has would leave nothing; showing its age lets the owner weigh it. |
| Iron and copper suggestions | Nutrient amount only (0.4–0.8 oz Fe foliar, 0.25–0.5 oz Cu per 1,000 sq ft); ferrous sulfate never called chelated iron | Amount of ferrous / copper sulfate; no amount at all | Both salts are also pesticide actives (ferrous sulfate for moss), and a pesticide's amount is the label's (HLD Non-Goals, "Label rates"). The nutrient amounts restate the former salt amounts, so the agronomy is unchanged. Chelated iron is a different material. |
| Ca / Mg / S label | "Secondary & micronutrients" | "Micronutrients"; separate Ca/Mg/S section | Calcium, magnesium and sulfur are secondary macronutrients; one combined heading is accurate without splitting a short table. |
| Magnesium need (when in the feed) | Either-or: a Supplement targeting Magnesium or dolomitic lime | One need requiring both target and amendment kind; Supplement only | Both materials correct low magnesium; requiring both axes on one product matches nothing. |
| Amounts in soil-test advice | Agronomic amounts per 1,000 sq ft: lime or sulfur for pH, lb N / P₂O₅ / K₂O for N-P-K; never a product's label rate | Name the material only; defer to the label or the lab report; quote typical label rates | A pH or nutrient correction cannot be expressed without an amount, and that amount is agronomy, independent of the product. How much of a given product to apply is set by its label, the legal authority. Converting a nutrient amount into pounds of a fertilizer or amendment material (`product_rate_lbs_per_1000sqft`, the suggestion strings) is still agronomy; a material that is also a pesticide active gets only the nutrient amount. |

## Open Questions & Future Decisions

### Resolved
1. ✅ Advice reads only the latest test by `test_date`; older tests are history.
2. ✅ Soil-test advice is not part of the recommendation feed; the Soil Tests page and MCP `soil_test_advice` are its surfaces. It can rejoin the feed once testing is regular, as a source ahead of the nitrogen guard, inventory enrichment and answered state (Feed Integration).
3. ✅ Lime or sulfur amounts for pH and lb N / P₂O₅ / K₂O per 1,000 sq ft for N-P-K are designed agronomic amounts, not label rates (HLD Non-Goals, "Label rates").
4. ✅ In the feed, `soil_test_npk` is a feeding only when it asks for nitrogen. The guard holds it whole under a block or heat stress and caps only its nitrogen at the budget, so P/K advice and its need stand.
5. ✅ TurfOps serves cool-season lawns; soil-test behavior for a warm-season profile is not designed.
6. ✅ A nutrient amount may also be stated as an amount of a fertilizer or amendment material: `product_rate_lbs_per_1000sqft` (pounds of an example analysis; `soil_test_recommendations.rs:210-213`, `SoilTests.tsx:606-611`) and the material amounts in the nutrient suggestions. A material that is also a pesticide active (copper sulfate, ferrous sulfate) gets only the nutrient amount (HLD Non-Goals, "Label rates").

### Deferred
1. **Magnesium need depends on need alternatives** (SOIL-REJOIN-007, deferred). The either-or magnesium need requires Product inventory's need alternatives (INV-NEED-006), which `ProductNeed` does not support; one need ANDing a Supplement target and a SoilAmendment kind would match no product. It must exist before soil-test advice rejoins the feed.
2. **Answers would survive a new test** (applies only if soil-test advice rejoins the feed). With fixed ids and `SoilTest` in the 120-day seasonal bucket (`models/recommendation_state.rs:46`), dismissing `soil_test_ph` would hide the advice from a newer test for up to 120 days at the same severity.
3. **Buffer pH lime amount** (SOIL-PH-007, deferred). Needs a record of the lab's buffer (SMP, Mehlich, Sikora) and a published calibration for each before a reported buffer pH can set the lime amount. `organic_matter_pct` and `cec` also remain stored but unused by advice.
4. **Validation limits.** Implausibly large values are accepted, and PUT/DELETE are not scoped to the profile.
5. **Placement:** the N-P-K evaluator mixes classification and the nothing-to-apply branch; ratio, example analysis and wording are in `soil_test_recommendations/ratio.rs`.
6. **File size:** `frontend/src/pages/SoilTests.tsx` is 570 lines with `SoilTests` well over 50 lines; `calculate_npk_recommendation` is over 50 lines; `api/soil_tests.rs` repeats the profile lookup across handlers.
7. **Guard on a P/K-only recommendation** (applies only if soil-test advice rejoins the feed). Intent (Resolved #4): at the budget the guard caps only nitrogen, so P₂O₅ / K₂O advice and its need stand, and a P/K-only `soil_test_npk` is not a feeding. The guard's "target reached" branch clears the whole action and every need (`nitrogen_guard.rs:102-112`, RULE-GUARD-005 deferred); if it covered `soil_test_npk` as built, P/K advice would be lost.
8. **MCP doctrine wording.** `mcp/instructions.rs:23-24` forbids any rate other than a stored label rate, which an agent can read as forbidding the agronomic amounts `soil_test_advice` returns. Owned by MCP server; aligning it to the HLD's amount rule is a cross-segment change.

## References

- `docs/high-level-design.md` — Non-Goals ("Label rates", "Warm-season lawns"), One recommendation feed, Key Design Decisions (agronomic amounts; server-local "today"), Success Metrics
- `backend/src/models/soil_test.rs`, `backend/src/api/soil_tests.rs`, `backend/src/db/soil_test_queries.rs`, `backend/src/db/migrations/20240101000007_soil_tests.sql`
- `backend/src/logic/soil_test_recommendations.rs`, `backend/src/logic/soil_test_thresholds.rs`
- `backend/src/models/recommendation_state.rs` (episode policy), `backend/src/logic/rules/nitrogen_guard.rs`, `backend/src/models/nitrogen_budget.rs`
- `backend/src/models/inventory.rs` (`ProductNeed::fits`), `backend/src/models/product/profile.rs` (`scoped_to_category`)
- `backend/src/mcp/tools/log.rs`, `backend/src/mcp/instructions.rs`
- `frontend/src/pages/SoilTests.tsx`, `frontend/src/types/index.ts`, `frontend/src/api/client.ts`
- `docs/plans/product-inventory.md` — soil-test need table (planned P/K Supplement need)
- Sibling LLDs: recommendation-feed (FEED), product-inventory (INV), applications (APP), agronomic-rules (RULE), mcp-server (MCP), web-ui (UI)
