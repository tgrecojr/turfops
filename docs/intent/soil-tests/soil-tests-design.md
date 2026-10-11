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

**Divergences from intent, flagged up front.**

- The feed adds `soil_test_ph`, `soil_test_npk` and `soil_test_micro_*` recommendations (`backend/src/api/recommendation_feed.rs:70-79`); intent is that it adds none (Feed Integration; Deferred #18).
- While they are in the feed, `soil_test_npk` escapes the nitrogen guard (Deferred #3) and the magnesium need cannot be met by any product (Deferred #2, SOIL-REJOIN-007). Both are dormant once #18 is fixed.

## Data Model and Persistence

Owned table: `soil_tests` (`backend/src/db/migrations/20240101000007_soil_tests.sql`).

| Column | Type | Notes |
|---|---|---|
| `id` | `BIGSERIAL` PK | |
| `lawn_profile_id` | `BIGINT NOT NULL` → `lawn_profiles(id) ON DELETE CASCADE` | Always the default (first) profile |
| `test_date` | `DATE NOT NULL` | Defines "latest" |
| `lab_name` | `TEXT` | Optional |
| `ph` | `DOUBLE PRECISION NOT NULL` | The only required measurement |
| `buffer_ph` | `DOUBLE PRECISION` | When reported, sets the lime amount (pH correction) |
| `phosphorus_ppm`, `potassium_ppm` | `DOUBLE PRECISION` | Drive the N-P-K advice |
| `calcium_ppm`, `magnesium_ppm`, `sulfur_ppm` | `DOUBLE PRECISION` | Secondary nutrients, evaluated with the micronutrients |
| `iron_ppm`, `manganese_ppm`, `zinc_ppm`, `boron_ppm`, `copper_ppm` | `DOUBLE PRECISION` | Micronutrients |
| `organic_matter_pct`, `cec` | `DOUBLE PRECISION` | Stored and shown; not used by advice |
| `notes` | `TEXT` | |
| `created_at` | `TIMESTAMPTZ DEFAULT NOW()` | Never updated |

Index `idx_soil_tests_profile_date (lawn_profile_id, test_date DESC)` serves both the list and the latest-test lookup.

The Rust model is `SoilTest` in `backend/src/models/soil_test.rs`; queries live in `backend/src/db/soil_test_queries.rs` (a private `SoilTestRow` maps rows to the model).

**Which test is "latest".** The latest test is the one with the newest `test_date`. When several share that date, the most recently entered one (newest `created_at`, then highest `id`) is latest, and the history list orders same-date tests the same way, so its first row is always the test the advice reads (SOIL-DATA-005). A second same-day entry is usually a corrected re-entry or a later lab report, so it reflects the owner's latest knowledge. **Divergence:** `get_latest_soil_test` orders by `test_date DESC LIMIT 1` alone (`soil_test_queries.rs:37`), and the list by `test_date DESC` alone (`:18`), so either same-date test can drive advice (Deferred #5).

## API Endpoints

All handlers are in `backend/src/api/soil_tests.rs`, routed in `backend/src/main.rs`. Every handler first resolves the default lawn profile (`queries::get_default_lawn_profile`, the lowest `id`) except DELETE.

| Method | Path | Request | Response | Errors |
|---|---|---|---|---|
| GET | `/api/v1/soil-tests` | `?limit=` (default 50, clamped 1–200), `?offset=` (≥0) | `SoilTest[]`, newest `test_date` first | 404 no profile |
| POST | `/api/v1/soil-tests` | `CreateSoilTestRequest` (`test_date` `YYYY-MM-DD`, `ph` required, all else optional) | 201 + created `SoilTest` | 400 bad date; 400 `pH must be between 0 and 14`; 400 negative measured value; 404 no profile |
| PUT | `/api/v1/soil-tests/{id}` | same body (full replace of every user field) | 200 + updated row as stored | 400 as POST; 404 `Soil test {id} not found` |
| DELETE | `/api/v1/soil-tests/{id}` | — | 204 | 404 `Soil test {id} not found` |
| GET | `/api/v1/soil-tests/recommendations` | — | `SoilTestSummary` for the latest test | 404 `No soil tests found`; 404 no profile |

Validation covers the date format, `0 ≤ pH ≤ 14`, and non-negative measured values: buffer pH, every ppm value, organic matter % and CEC are concentrations or indices that cannot be negative, so a negative one is a typo and is rejected with 400 and a plain message naming the field (SOIL-API-011). Deleting an id that does not exist is 404, like PUT (SOIL-API-012). **Divergence:** the handlers check only the date and pH, and DELETE returns 204 for a missing id (`api/soil_tests.rs:181-187`; Deferred #10). PUT and DELETE are keyed by `id` alone (the profile is not part of the `WHERE`), which is sound only under the single-lawn assumption.

`SoilTestSummary` (`models/soil_test.rs`) carries the test itself, `ph_recommendation: PhRecommendation | null`, `npk_recommendation: NpkRecommendation | null` and `micronutrient_recommendations[]`. The frontend mirrors these in `frontend/src/types/index.ts`.

**Boundary — nitrogen budget.** The recommendations endpoint loads this calendar year's applications (`Local::now().year()`, 1 Jan to 1 Jan, via `queries::get_applications_for_profile_in_range`) and sums `Application::turf_nitrogen_lbs()` (owned by Applications) against `annual_n_target(grass_type).recommended_lbs_per_1000sqft` (`backend/src/models/nitrogen_budget.rs`). Plant applications contribute nothing because `turf_nitrogen_lbs` excludes them.

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

**Buffer pH sets the lime amount when the lab reports it** (SOIL-PH-007). Water pH says how acidic the soil is; buffer pH measures its reserve acidity, which is what governs how much lime it takes to move pH. The soil-type factor is only a proxy for that reserve. So when the test carries a `buffer_ph` and lime is called for, the lime amount comes from the buffer pH and the target pH instead of the soil-type factor, under the same per-treatment cap and split note, and the explanation says the amount is based on the buffer pH. The buffer calibration must match the buffer the owner's lab uses (SMP, Mehlich and Sikora buffers read differently). Sulfur, which lowers pH, is unaffected. **Divergence:** `calculate_ph_recommendation` ignores `buffer_ph` (Deferred #9).

### N-P-K ratio — `calculate_npk_recommendation`

| Nutrient | Low | Adequate | High |
|---|---|---|---|
| Phosphorus (Mehlich-3 ppm) | < 15 | 15–50 | > 50 |
| Potassium (Mehlich-3 ppm) | < 100 | 100–175 | > 175 |
| Not reported | **Not tested**: no P₂O₅ / K₂O advised for it | | |

Per-nutrient intent, in lb of nutrient per 1,000 sq ft: P₂O₅ is supplied only when P is Low; K₂O is supplied when K is Low (more) or Adequate (maintenance), never when High or Not tested; N is supplied from the remaining annual budget, capped per application and zeroed when the remainder falls below a small floor. When none of N, P or K is wanted the function still returns `Some` with ratio `"N/A"`, all amounts zero and an explanation that no fertilizer is recommended. Otherwise `suggest_ratio` normalizes the three amounts to the smallest non-zero one to produce a ratio string (e.g. `3-1-2`) and maps the N/P/K presence pattern to one hard-coded "common commercial" analysis (seven fixed analyses such as a straight-N, a P-only, a K-only and several complete blends). `product_rate_lbs_per_1000sqft` back-calculates the pounds of that example analysis from its N (or P, then K) percentage, and the explanation and feed action state it (an agronomic amount; Resolved #6). The explanation names the P/K levels and ppm, says which of P and K was not tested, the remaining budget, a budget-cap note when little remains, and the Loam-default note.

**Unreported P or K is "Not tested"** (SOIL-NPK-012, SOIL-NPK-013). Advising maintenance potash for a soil whose potassium was never measured, or describing an unmeasured nutrient as adequate, would be an invented answer. A test without P/K still gets nitrogen advice from the budget; it gets no P₂O₅ or K₂O, and its explanation names the untested nutrients instead of an empty list of levels. **Divergence:** `classify_phosphorus` / `classify_potassium` map `None` to Adequate (`soil_test_recommendations.rs:280,289`), so a pH-only test still yields maintenance K₂O and an explanation reading "Your soil test shows ." (Deferred #4).

The evaluator itself has no seasonal or weather gate: it does not look at the date, the grass's feeding calendar, or the active `fertilizer_block` / heat-stress state. On the page and through MCP the advice is what the test calls for, not a call to act today; weather and budget holds belong to the nitrogen guard in the feed and would apply only if soil-test advice rejoins it (Feed Integration).

### Nutrient deficiencies — `evaluate_micronutrients`

Calcium, magnesium and sulfur are secondary nutrients, not micronutrients; they are evaluated in the same list, which the page titles "Secondary & micronutrients" (the API field keeps the name `micronutrient_recommendations`). Each reported nutrient below its threshold yields one `MicronutrientRecommendation` with `level: Low` and a fixed suggestion string. A nutrient at or above its threshold, or not reported, yields nothing. `High` is never produced here.

| Nutrient | Low below (ppm) | Suggested material (amount text omitted here) |
|---|---|---|
| Calcium | 500 | Gypsum (calcium sulfate) |
| Magnesium | 50 | Dolomitic lime (also raises pH) or Epsom salt |
| Sulfur | 10 | Elemental sulfur, or ammonium sulfate fertilizer |
| Iron | 5 | Iron as a foliar application, iron amount only |
| Manganese | 5 | Manganese sulfate |
| Zinc | 1.0 | Zinc sulfate |
| Boron | 0.5 | Borax, with a toxicity caution |
| Copper | 0.5 | Copper amount only |

Every suggestion contains a fixed amount per 1,000 sq ft, not computed from the test (`soil_test_recommendations.rs:242-251`). Gypsum and elemental sulfur sit beside lime as soil-correcting materials; Epsom salt, manganese sulfate, zinc sulfate and borax are fertilizer or amendment materials, so an amount of each is an agronomic amount. Copper sulfate and ferrous sulfate are also registered pesticide actives (ferrous sulfate for moss control), so under the HLD's label-rate rule the copper and iron suggestions give only the nutrient amount, never an amount of the salt (SOIL-NUTR-004, SOIL-NUTR-005). Ferrous sulfate is not a chelated iron, and the iron suggestion never calls it one. **Divergence:** the copper string states an amount of copper sulfate, and the iron string reads "chelated iron (ferrous sulfate)" with an amount of it (`soil_test_recommendations.rs:242-251`; Deferred #1).

## Feed Integration

**Intent: soil-test advice is not in the recommendation feed.** The Soil Tests page and the MCP `soil_test_advice` tool are its only surfaces. The feed (dashboard alerts, the Recommendations page, the shopping list, the MCP `recommendations` and `lawn_snapshot` tools) carries no soil-test recommendation, so no soil-test product need reaches the inventory pass or the shopping list. The reason is procedural, not technical: the owner tests soil too rarely for a single, possibly old test to drive standing advice.

**Divergence.** `recommendation_feed::all` appends soil-test recommendations after rules, disease, timing, plant tasks and follow-ups, and before inventory enrichment and persisted answers (`backend/src/api/recommendation_feed.rs:70-79`), through the adapter `soil_test_recommendations` and the need builders `amendment_need` / `micronutrient_need` (`recommendation_feed.rs:99-259`). The latest test is read with `if let Ok(Some(test))`, so no test or a database error adds nothing. The applications passed in are the feed's rule history filtered to the current year. What it emits today:

| Id | Severity | Fires when | Declared `ProductNeed` |
|---|---|---|---|
| `soil_test_ph` | **Warning** when `|pH − target| > 1.0`, else **Advisory** | pH recommendation present | Lime: `SoilAmendment`, kinds `CalciticLime` \| `DolomiticLime`. Sulfur: `SoilAmendment`, kind `Sulfur` |
| `soil_test_npk` | **Advisory** | Any of the N/P/K amounts > 0 | `Fertilizer` (`"{ratio} fertilizer"`), no NPK constraint |
| `soil_test_micro_{nutrient}` | **Info** | Each Low nutrient | Calcium → `SoilAmendment` `Gypsum`; Magnesium → see Deferred #2; Sulfur → `SoilAmendment` `Sulfur`; Iron / Manganese / Zinc / Boron / Copper → `Supplement` with that `ProductTarget` |

All are category `SoilTest` (the answered-state policy's 120-day seasonal bucket). Each carries data points and an `action` that states an amount; `soil_test_npk` adds its own "N budget remaining" point (`recommendation_feed.rs:172-178`).

**Rejoining the feed.** Soil-test advice can return to the feed once the owner tests on a regular schedule, so the latest test is reasonably current. It then enters as a feed source like the others, ahead of the nitrogen guard, inventory enrichment and answered state, with the ids, severities and needs above, and these rules apply:

- `soil_test_npk` is marked as a feeding only when it asks for nitrogen (N > 0). A P/K-only recommendation is not a feeding.
- Under a fertilizer block or Warning+ heat stress the guard holds the whole recommendation: no fertilizer of any kind goes down.
- When the annual nitrogen budget is reached or limits the amount, the guard caps only the nitrogen part; P₂O₅ and K₂O advice stands, with its need.
- The guard is the only author of the "N budget remaining" data point; the soil-test recommendation does not add its own.
- The magnesium need is either-or: a Supplement targeting Magnesium or a SoilAmendment of kind dolomitic lime satisfies it, matched by Product inventory (SOIL-REJOIN-007). Dolomitic lime supplies magnesium while raising pH; a magnesium supplement such as Epsom salt supplies it without moving pH. Either answers "do I have something for low magnesium".

The guard's behavior is owned by Agronomic rules and its placement by Recommendation feed; this component owns marking `soil_test_npk`.

## Degradation and Empty States

| Situation | Endpoint | Page |
|---|---|---|
| No lawn profile | 404 on every GET/POST/PUT | Error banner |
| No soil tests | `/recommendations` → 404 `No soil tests found` | Recommendations panel hidden (page skips the call); "No soil tests recorded yet" in history |
| DB error reading latest test | 500 | Error banner |
| pH within dead band, nothing low, K High, N budget spent | Summary with `ph: null`, NPK "N/A", empty list | "No fertilizer needed" with the explanation (see UI) |
| Profile soil type unset | Loam assumed, noted in text | same |

The feed carries no soil-test advice, so none of these situations affects it.

## Soil Tests Page

`frontend/src/pages/SoilTests.tsx` (777 lines), route `/soil-tests` (`frontend/src/App.tsx`), nav label "Soil Tests". API calls in `frontend/src/api/client.ts` (`getSoilTests`, `createSoilTest`, `updateSoilTest`, `deleteSoilTest`, `getSoilTestRecommendations`).

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
- **Recommendations panel.** Headed by the date of the test the advice is based on and its age (time from `test_date` to today's local date). When that test is more than 3 years old the panel suggests retesting: extension guidance for turf is to test every 2–3 years, and beyond that the advice describes a soil that may have changed (SOIL-UI-012, SOIL-UI-014, SOIL-UI-015). pH card (current, target, amendment, amount, explanation); Fertilizer card (ratio, N/P₂O₅/K₂O amounts with P and K `LevelBadge`s, example analysis with amount, N budget remaining, explanation); deficiency table titled "Secondary & micronutrients" (nutrient, current, threshold, level, suggestion; SOIL-UI-018). `LevelBadge` renders the level word (Low, Adequate, High, Not tested) in a status color (`NUTRIENT_LEVEL_COLORS`), never color alone.
- **Nothing to apply.** When the N-P-K recommendation advises no N, P₂O₅ or K₂O, the Fertilizer card gives way to "No fertilizer needed" with the explanation and the remaining nitrogen, never a ratio of "N/A" and zero amounts (SOIL-UI-016). The backend always returns an `npk_recommendation`, so this, not an empty panel, is how a fully adequate soil reads.
- **History table** shows a subset of fields (no buffer pH, S, or micronutrients). Delete asks for confirmation naming the test's date before deleting (SOIL-UI-017); a deleted test cannot be recovered and may be the one driving the advice.
- The page shows no shelf status: soil-test advice is not in the feed, so the inventory pass never sees it.
- **Divergence:** the page names neither the test's date nor its age and never suggests retesting; a fully adequate soil shows a Fertilizer card reading "N/A" with zero amounts while the panel's "All soil nutrient levels are adequate" branch is unreachable (`SoilTests.tsx:512-517`); Delete deletes at once; the deficiency table is titled "Micronutrient Deficiencies"; `LevelBadge` has no "Not tested" (Deferred #6, #11, #14).

## MCP Exposure

The MCP server (MCP) exposes `soil_tests` (limit default 5, clamped 1–50) and `soil_test_advice` (the recommendations endpoint, untrimmed) in `backend/src/mcp/tools/log.rs`. Both return what the REST handlers return, amounts included. The MCP doctrine (`backend/src/mcp/instructions.rs:21-22`, "Never give an application rate except a product's stored `label_rate`") is worded more strictly than the HLD's amount rule and reads as forbidding the lime, sulfur and nutrient amounts this tool returns; the doctrine text belongs to MCP server (Deferred #17).

## Decisions & Alternatives

| Decision | Chosen | Alternatives Considered | Rationale |
|---|---|---|---|
| Which test drives advice | Newest `test_date` only; among tests sharing that date, the most recently entered | Blend or trend several tests; newest `created_at` overall; an arbitrary pick on a same-date tie | Labs report a point-in-time state; the newest sample supersedes older ones, and the test date (not entry date) is when it was true. On a same-date tie the later entry is usually a correction or a later report, so it carries the owner's latest knowledge; an arbitrary pick would let advice flip between reads. |
| Persistence of advice | Recompute on every read | Store advice with the test | Advice depends on the profile and this year's log, which change independently of the test. [inferred] |
| pH target | 6.5 for cool-season lawns (incl. Mixed), ±0.2 dead band | Per-species targets; a target range | Matches common extension guidance for cool-season turf; the dead band avoids advising on lab noise. [inferred] |
| Nutrient interpretation basis | Fixed Mehlich-3 ppm thresholds | Lab-supplied interpretation; per-lab extractant | One extractant keeps the thresholds simple; Mehlich-3 is the common mid-Atlantic extractant. [inferred] |
| Missing P / K | "Not tested": no P₂O₅ / K₂O for it, and the advice says it was not tested; N still advised from the budget | Treat as Adequate (maintenance K₂O); skip the whole N-P-K advice | Treating an unmeasured nutrient as adequate invents a reading and advises potash nobody showed a need for. Skipping the whole advice would lose the nitrogen side, which rests on the budget, not the test. |
| N in soil-test fertilizer advice | Tied to this calendar year's (server-local) remaining annual N budget | Independent N recommendation | Keeps soil-test advice consistent with the nitrogen budget the rules use. [inferred] |
| Soil-test advice in the feed | Not in the feed: the Soil Tests page and MCP `soil_test_advice` are its only surfaces; it can rejoin once testing is regular | A feed source like the others; in the feed only while the latest test is recent | The owner tests soil rarely. A single, possibly old test should not drive standing alerts, shopping-list items or agent advice; the page and the tool still show what the test calls for whenever the owner looks. A recency cutoff would still let one test speak for a year of alerts. |
| Nitrogen guard over `soil_test_npk` (when in the feed) | Marked as a feeding only when it asks for N > 0; held whole under a fertilizer block or Warning+ heat stress; at the budget only its nitrogen is capped and P₂O₅ / K₂O stand; the guard alone writes "N budget remaining" | Exempt from the guard; the evaluator re-implementing block and heat checks; dropping the whole recommendation at the budget | Under a block or heat stress no fertilizer should go down, whatever its analysis. The budget limits nitrogen only, so phosphorus and potassium the soil lacks are still worth applying. One author for the budget point keeps one figure per recommendation. |
| Feed ids (when in the feed) | Fixed ids (`soil_test_ph`, `soil_test_npk`, `soil_test_micro_<x>`) | Ids including the test id | The answered-state policy expires answers by episode, so ids need not encode the test. [inferred] |
| Feed severities (when in the feed) | pH Warning beyond 1.0 units else Advisory; N-P-K Advisory; deficiencies Info | Uniform severity | A large pH gap limits nutrient availability broadly; single-nutrient lows are informational. [inferred] |
| Product needs (when in the feed) | Lime/sulfur kinds for pH; any Fertilizer for N-P-K; gypsum for Ca; per-nutrient Supplement for micros | Encode NPK ratios in the need; a second Supplement need when only P or K is short (as planned in `docs/plans/product-inventory.md`) | Shelf facts carry NPK, but matching on ratio would rarely find a product; category-level need answers "do I have a fertilizer". The planned P/K Supplement need is not implemented. [inferred] |
| Lime amount basis | Buffer pH when the lab reports it; else the soil-type factor | Always the soil-type factor; require buffer pH | Buffer pH measures the reserve acidity that sets lime requirement; soil type is a proxy for it. Many tests carry only water pH, so the factor stays as the fallback. |
| Stale tests | Show the test's age; suggest retesting past 3 years; advice still shown | Hide advice past an age; no age shown | Extension guidance for turf is a test every 2–3 years. Hiding the only test the owner has would leave nothing; showing its age lets the owner weigh it. |
| Iron and copper suggestions | Nutrient amount only; ferrous sulfate never called chelated iron | Amount of ferrous / copper sulfate | Both salts are also pesticide actives (ferrous sulfate for moss), and a pesticide's amount is the label's (HLD Non-Goals, "Label rates"). Chelated iron is a different material. |
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
6. ✅ A nutrient amount may also be stated as an amount of a fertilizer or amendment material: `product_rate_lbs_per_1000sqft` (pounds of an example analysis; `soil_test_recommendations.rs:210-213`, `recommendation_feed.rs:180-183`, `SoilTests.tsx:606-611`) and the material amounts in the nutrient suggestions. A material that is also a pesticide active (copper sulfate, ferrous sulfate) gets only the nutrient amount (HLD Non-Goals, "Label rates").

### Deferred
1. **Divergence: copper and iron suggestions state pesticide-active amounts** (SOIL-NUTR-004, SOIL-NUTR-005). The copper string gives an amount of copper sulfate and the iron string an amount of "chelated iron (ferrous sulfate)" (`soil_test_recommendations.rs:242-251`). Intent: each states only the nutrient amount, and ferrous sulfate is never called chelated iron. The other six suggestions and `product_rate_lbs_per_1000sqft` are designed (Resolved #6).
2. **Divergence: magnesium need cannot be met** by any product (SOIL-REJOIN-007). It is declared as one need ANDing a Supplement target and a SoilAmendment kind (Feed Integration). Intent: an either-or need, a Supplement targeting Magnesium or dolomitic lime, matched by Product inventory. Current behavior: a Low magnesium recommendation is always "need to buy" and always shops. Dormant once soil-test advice leaves the feed (#18): the need matters only to the inventory pass and the shopping list.
3. **`soil_test_npk` bypasses the nitrogen guard** (dormant; superseded by #18). The feed appends it after the guard has run inside the rules engine (`engine.rs:49-54`, `nitrogen_guard.rs:12-17`, `recommendation_feed.rs:70-79`), so today it advises feeding at Advisory during a fertilizer block or Warning+ heat stress and is never capped. Once soil-test advice leaves the feed this has no effect; if it rejoins, it enters before the guard under the rules in Feed Integration. Nothing gates it by season either, which matters only in the feed.
4. **Divergence: missing P/K is treated as Adequate** (SOIL-NPK-012, SOIL-NPK-013, SOIL-UI-009). `soil_test_recommendations.rs:280,289`. Current behavior: a pH-only test still yields maintenance K and an explanation reading "Your soil test shows ." with nothing listed (`:179-191`, `:211`). Intent: an unreported P or K is Not tested, gets no P₂O₅ / K₂O, and the explanation says it was not tested.
5. **Divergence: no tie-breaker for "latest"** (SOIL-DATA-005). Current behavior: the query orders only by `test_date DESC LIMIT 1` (`soil_test_queries.rs:37`), so either same-date test can drive advice. Intent: the most recently entered same-date test is latest, and the list orders ties the same way.
6. **Divergence: no age or retest reminder** (SOIL-UI-014, SOIL-UI-015). Current behavior: a years-old test drives advice with no age shown and no retest suggestion. Intent: the page shows the test's age and suggests retesting once it is over 3 years old.
7. **Answers survive a new test** (feed only; dormant once #18 is fixed). Current behavior: ids are fixed and `SoilTest` falls into the 120-day seasonal bucket (`models/recommendation_state.rs:46`), so dismissing `soil_test_ph` hides the advice from a newer test for up to 120 days at the same severity.
8. **Feed swallows DB errors** for the latest-test read (dormant once #18 is fixed). Current behavior: `if let Ok(Some(..))` (`recommendation_feed.rs:72`) silently drops the advice on a transient error, while the plant and product reads beside it propagate errors with `?`.
9. **Divergence: buffer pH is ignored** (SOIL-PH-007). Current behavior: the lime amount always comes from the soil-type factor. Intent: a reported buffer pH sets the lime amount, with the calibration matched to the owner's lab's buffer. `organic_matter_pct` and `cec` remain stored but unused by advice.
10. **Divergence: validation gaps** (SOIL-API-011, SOIL-API-012). Current behavior: negative measured values are accepted and DELETE of a missing id returns 204 (`api/soil_tests.rs:181-187`). Intent: a negative value is 400 with a plain message naming the field; deleting a missing test is 404. Still open: implausibly large values are accepted, and PUT/DELETE are not scoped to the profile.
11. **Divergence: Soil Tests page states** (SOIL-UI-012, SOIL-UI-016, SOIL-UI-017). Current behavior: a fully adequate soil shows an "N/A" Fertilizer card because the backend always returns an `npk_recommendation` and the page's "All adequate" branch is unreachable (`soil_test_recommendations.rs:130-155`, `SoilTests.tsx:512-517`); Delete has no confirmation; the page does not name the test the advice is based on. Intent: "No fertilizer needed", a confirmation before delete, and the test's date beside the advice.
12. **Placement:** the feed adapter and need builders belong in soil-tests code beside the logic but live in `api/recommendation_feed.rs`; the N-P-K evaluator mixes classification, ratio search and product back-calculation.
13. **File size:** `logic/soil_test_recommendations.rs` is 525 lines (over the 300-line limit; about 155 are tests) and `calculate_npk_recommendation` is ~140 lines (over the 50-line limit); `frontend/src/pages/SoilTests.tsx` is 777 lines with `SoilTests` and `SoilTestRecommendationsPanel` both well over 50 lines; `api/soil_tests.rs` repeats profile lookup, date parsing and pH validation across handlers.
14. **Divergence: calcium, magnesium and sulfur are labelled "micronutrients"** (SOIL-UI-018). The page titles their table "Micronutrient Deficiencies" although they are secondary macronutrients. Intent: "Secondary & micronutrients". The API field name `micronutrient_recommendations` and the feed ids `soil_test_micro_*` are unaffected.
15. **Budget basis differs from the guard's.** Current behavior: the soil-test N side counts every application dated in the current year, future-dated ones included (`recommendation_feed.rs:75`, `api/soil_tests.rs:206-208`), while the guard counts only those on or before today (`nitrogen_guard.rs:32-38`). The page's remaining budget can therefore differ from the feed's. If soil-test advice rejoins the feed, its N amount and the guard's "N budget remaining" point would rest on different bases; the evaluator should adopt the guard's.
16. **Guard on a P/K-only recommendation** (dormant while #18 holds). Intent (Resolved #4): at the budget the guard caps only nitrogen, so P₂O₅ / K₂O advice and its need stand, and a P/K-only `soil_test_npk` is not a feeding. Today's guard, at "target reached", clears the whole action and every need (`nitrogen_guard.rs:64-74`); if it covered `soil_test_npk` as built, P/K advice would be lost.
17. **MCP doctrine wording.** `mcp/instructions.rs:21-22` forbids any rate other than a stored label rate, which an agent can read as forbidding the agronomic amounts `soil_test_advice` returns. Owned by MCP server; aligning it to the HLD's amount rule is a cross-segment change.
18. **Soil-test advice is in the feed** (divergence). `recommendation_feed::all` appends `soil_test_ph`, `soil_test_npk` and `soil_test_micro_*` (`recommendation_feed.rs:70-79`, adapter `:99-259`), so they reach dashboard alerts, the Recommendations page, the shopping list and the MCP `recommendations` / `lawn_snapshot` tools. Intent: the feed carries no soil-test advice. Removing the call is a Recommendation feed change; the adapter can stay for a later rejoin (#12).

## References

- `docs/high-level-design.md` — Non-Goals ("Label rates", "Warm-season lawns"), One recommendation feed, Key Design Decisions (agronomic amounts; server-local "today"), Success Metrics
- `backend/src/models/soil_test.rs`, `backend/src/api/soil_tests.rs`, `backend/src/db/soil_test_queries.rs`, `backend/src/db/migrations/20240101000007_soil_tests.sql`
- `backend/src/logic/soil_test_recommendations.rs`, `backend/src/logic/soil_test_thresholds.rs`
- `backend/src/api/recommendation_feed.rs` (`soil_test_recommendations`, `amendment_need`, `micronutrient_need`)
- `backend/src/models/recommendation_state.rs` (episode policy), `backend/src/logic/rules/nitrogen_guard.rs`, `backend/src/models/nitrogen_budget.rs`
- `backend/src/models/inventory.rs` (`ProductNeed::fits`), `backend/src/models/product/profile.rs` (`scoped_to_category`)
- `backend/src/mcp/tools/log.rs`, `backend/src/mcp/instructions.rs`
- `frontend/src/pages/SoilTests.tsx`, `frontend/src/types/index.ts`, `frontend/src/api/client.ts`
- `docs/plans/product-inventory.md` — soil-test need table (planned P/K Supplement need)
- Sibling LLDs: recommendation-feed (FEED), product-inventory (INV), applications (APP), agronomic-rules (RULE), mcp-server (MCP), web-ui (UI)
