# Soil Tests — EARS Specs

Design: [soil-tests-design.md](soil-tests-design.md). Prefix `SOIL`. Facets: `DATA` (stored tests and which one is latest), `API` (soil-test endpoints), `ADV` (rules every piece of soil-test advice follows), `PH` (pH correction), `NPK` (N-P-K advice and the nitrogen budget it reads), `NUTR` (nutrient deficiencies), `UI` (Soil Tests page), `REJOIN` (rules that apply if soil-test advice returns to the recommendation feed).

"Latest test" is the lawn's soil test with the newest `test_date`. "Soil-test advice" is the `SoilTestSummary` derived from it: an optional pH recommendation, an N-P-K recommendation and a list of nutrient deficiencies. Amounts are per 1,000 sq ft. Soil-test advice is not part of the recommendation feed (FEED-PIPE-018).

## Stored tests

- [x] **SOIL-DATA-001**: The system shall store each soil test against the default lawn profile with a test date, a required pH, and optional lab name, buffer pH, Mehlich-3 ppm values for phosphorus, potassium, calcium, magnesium, sulfur, iron, manganese, zinc, boron and copper, organic matter %, CEC and notes.
- [x] **SOIL-DATA-002**: The system shall delete a lawn profile's soil tests when that profile is deleted.
- [x] **SOIL-DATA-003**: The system shall treat the soil test with the newest `test_date` (the date the sample was true, not the date it was entered) as the latest test.
- [x] **SOIL-DATA-004**: The system shall persist no soil-test advice; every read of soil-test advice shall derive it afresh from the latest test, the lawn profile and the application log, so editing or deleting a test changes the next read.
- [ ] **SOIL-DATA-005**: When two or more soil tests share the newest `test_date`, the system shall treat the one entered most recently (newest `created_at`, then highest id) as the latest test, and the soil-test list shall order same-date tests the same way.

## Endpoints

- [x] **SOIL-API-001**: When `GET /api/v1/soil-tests` is called, the system shall return the default profile's soil tests newest `test_date` first, paged by `?limit=` (default 50, clamped to 1–200) and `?offset=` (default 0, never below 0).
- [x] **SOIL-API-002**: When `POST /api/v1/soil-tests` is called with a valid body, the system shall store the test and return 201 with the stored test, including its id.
- [x] **SOIL-API-003**: When `PUT /api/v1/soil-tests/{id}` is called with a valid body, the system shall replace every user field of that test and return 200 with the test as stored.
- [x] **SOIL-API-004**: If `PUT /api/v1/soil-tests/{id}` names no stored test, then the system shall return 404 with "Soil test {id} not found".
- [x] **SOIL-API-005**: When `DELETE /api/v1/soil-tests/{id}` deletes a stored test, the system shall return 204.
- [x] **SOIL-API-006**: If a soil-test create or update body has a `test_date` that is not `YYYY-MM-DD`, then the system shall return 400 with a message naming the bad value and the expected `YYYY-MM-DD` format.
- [x] **SOIL-API-007**: If a soil-test create or update body has a pH below 0 or above 14, then the system shall return 400 with "pH must be between 0 and 14".
- [x] **SOIL-API-008**: When `GET /api/v1/soil-tests/recommendations` is called, the system shall return the soil-test advice for the latest test together with that test.
- [x] **SOIL-API-009**: If `GET /api/v1/soil-tests/recommendations` is called when no soil test is stored, then the system shall return 404 with "No soil tests found".
- [x] **SOIL-API-010**: If no lawn profile exists, then the soil-test list, create, update and recommendations endpoints shall return 404 with "No lawn profile found".
- [ ] **SOIL-API-011**: If a soil-test create or update body has a negative buffer pH, Mehlich-3 ppm value (phosphorus, potassium, calcium, magnesium, sulfur, iron, manganese, zinc, boron or copper), organic matter % or CEC, then the system shall return 400 with a plain message naming the field and saying it cannot be negative, and store nothing.
- [ ] **SOIL-API-012**: If `DELETE /api/v1/soil-tests/{id}` names no stored test, then the system shall return 404 with "Soil test {id} not found".

## Advice rules

- [x] **SOIL-ADV-001**: The system shall derive soil-test advice from the latest test only, never blending or trending older tests.
- [x] **SOIL-ADV-002**: The system shall name materials in soil-test advice (lime, elemental sulfur, a fertilizer analysis, gypsum, a nutrient salt) and never a brand or a shelf product.
- [x] **SOIL-ADV-003**: Soil-test advice shall state agronomic amounts per 1,000 sq ft (lb of lime or elemental sulfur; lb N, P₂O₅ and K₂O; and amounts of fertilizer or soil-amendment materials, including pounds of an example fertilizer analysis) and shall never state a product's label rate.
- [x] **SOIL-ADV-004**: Soil-test advice shall describe what the latest test calls for and shall not be gated by the date, season, weather, fertilizer block or heat stress.
- [x] **SOIL-ADV-005**: Soil-test advice shall treat a `Mixed` grass type as cool-season.

## pH correction

- [x] **SOIL-PH-001**: The soil-test pH evaluator shall use a target pH of 6.5 for cool-season lawns.
- [x] **SOIL-PH-002**: When the latest test's pH is within 0.2 of the target, the soil-test pH evaluator shall return no pH recommendation.
- [x] **SOIL-PH-003**: When the latest test reports no buffer pH and its pH is 0.2 or more below the target, the soil-test pH evaluator shall recommend "Lime (calcitic or dolomitic)" at the pH gap times the lime factor for the profile's soil type (lb per 1,000 sq ft per pH unit: Sandy 25, Sandy Loam 35, Loam 50, Silt Loam 60, Clay Loam 70, Clay 80), capped at 50 lb per 1,000 sq ft.
- [x] **SOIL-PH-004**: When the latest test's pH is 0.2 or more above the target, the soil-test pH evaluator shall recommend "Elemental sulfur" at the pH gap times the sulfur factor for the profile's soil type (lb per 1,000 sq ft per pH unit: Sandy 5, Sandy Loam 7, Loam 10, Silt Loam 12, Clay Loam 15, Clay 18), capped at 10 lb per 1,000 sq ft.
- [x] **SOIL-PH-005**: When the soil-test pH evaluator recommends lime or sulfur, its explanation shall state the current and target pH, the material and its amount, and that a need above the per-treatment cap is split into applications 3–6 months apart.
- [x] **SOIL-PH-006**: Where the lawn profile has no soil type, the soil-test pH evaluator (whenever it uses a soil-type factor) and the soil-test N-P-K evaluator shall assume Loam and say so in their explanations.
- [ ] **SOIL-PH-007**: When the latest test reports a buffer pH and its pH is 0.2 or more below the target, the soil-test pH evaluator shall compute the lime amount from the buffer pH and the target pH (the lab's measure of reserve acidity) in place of the soil-type factor, keep the 50 lb per 1,000 sq ft per-treatment cap, and say in its explanation that the amount is based on the buffer pH.

## N-P-K advice

- [x] **SOIL-NPK-001**: The soil-test N-P-K evaluator shall classify a reported Mehlich-3 phosphorus value as Low below 15 ppm, High above 50 ppm, else Adequate, and a reported potassium value as Low below 100 ppm, High above 175 ppm, else Adequate.
- [x] **SOIL-NPK-002**: The soil-test N-P-K evaluator shall advise 0.25 lb P₂O₅ per 1,000 sq ft when phosphorus is Low and none otherwise, and 0.5 lb K₂O when potassium is Low, 0.25 lb when Adequate and none when High.
- [x] **SOIL-NPK-003**: The soil-test N-P-K evaluator shall compute the remaining annual nitrogen as the grass type's recommended annual target (RULE-BUDGET-002) minus this calendar year's lawn nitrogen (`Application::turf_nitrogen_lbs()`, so plant applications count nothing), never below 0.
- [ ] **SOIL-NPK-004**: The soil-test N-P-K evaluator shall count toward this year's applied nitrogen only applications dated from Jan 1 of the server's local current year through the server's local today, the same basis as the nitrogen budget and the nitrogen guard (RULE-BUDGET-005).
- [x] **SOIL-NPK-005**: The soil-test N-P-K evaluator shall advise no nitrogen when the remaining annual nitrogen is below 0.25 lb N per 1,000 sq ft, and otherwise the remaining amount capped at 0.75 lb N per 1,000 sq ft.
- [x] **SOIL-NPK-006**: When the soil-test N-P-K evaluator advises no N, P₂O₅ or K₂O, it shall still return an N-P-K recommendation with ratio and example analysis "N/A", every amount 0, the remaining nitrogen, and an explanation that no fertilizer is recommended.
- [x] **SOIL-NPK-007**: When the soil-test N-P-K evaluator advises any nutrient, it shall state the ratio as the N, P₂O₅ and K₂O amounts each divided by the smallest non-zero amount and rounded (for example `3-1-2`).
- [x] **SOIL-NPK-008**: When the soil-test N-P-K evaluator advises any nutrient, it shall name one example fertilizer analysis by which nutrients are advised: N-P-K 24-4-12 when the N part is at least twice the K part, else 12-4-8; N-P only 15-5-0; N-K only 24-0-11 when the N part is at least twice the K part, else 16-0-8; N only 46-0-0; P-K only 0-5-10; P only 0-20-0; K only 0-0-50.
- [x] **SOIL-NPK-009**: When the soil-test N-P-K evaluator advises nitrogen, it shall state the pounds of the example analysis per 1,000 sq ft that supply the advised nitrogen.
- [ ] **SOIL-NPK-010**: When the soil-test N-P-K evaluator advises no nitrogen but advises P₂O₅ or K₂O, it shall state the pounds of the example analysis per 1,000 sq ft that supply the advised P₂O₅ (or, with no P₂O₅ advised, the advised K₂O), never 0.
- [x] **SOIL-NPK-011**: When the soil-test N-P-K evaluator advises any nutrient, its explanation shall state the reported P and K levels with their ppm, the remaining annual nitrogen, the advised nitrogen, the ratio, and the example analysis with its amount, adding a note that nitrogen is capped when less than 0.5 lb N per 1,000 sq ft remains.
- [ ] **SOIL-NPK-012**: When the latest test does not report phosphorus (or potassium), the soil-test N-P-K evaluator shall classify that nutrient as Not tested and advise none of it: no P₂O₅ for untested phosphorus, and no K₂O, maintenance included, for untested potassium.
- [ ] **SOIL-NPK-013**: When the latest test does not report phosphorus or potassium, the soil-test N-P-K evaluator's explanation shall say which of them was not tested and shall never state an empty list of soil-test levels.

## Nutrient deficiencies

- [x] **SOIL-NUTR-001**: The soil-test deficiency evaluator shall report a nutrient as Low, with its value, threshold and a suggestion, when the latest test reports it below its threshold in ppm: calcium 500, magnesium 50, sulfur 10, iron 5, manganese 5, zinc 1.0, boron 0.5, copper 0.5.
- [x] **SOIL-NUTR-002**: The soil-test deficiency evaluator shall report nothing for a nutrient that is at or above its threshold or that the latest test does not report, and shall never report a nutrient as High.
- [x] **SOIL-NUTR-003**: The soil-test deficiency evaluator shall suggest, each with an amount per 1,000 sq ft: gypsum for calcium; dolomitic lime (noting it also raises pH) or Epsom salt for magnesium; elemental sulfur or ammonium sulfate fertilizer for sulfur; manganese sulfate for manganese; zinc sulfate for zinc; borax with a caution that boron is toxic at low concentrations for boron.
- [ ] **SOIL-NUTR-004**: When the soil-test deficiency evaluator reports copper as Low, its suggestion shall state only the amount of copper per 1,000 sq ft and no amount of copper sulfate or any other material that is also a pesticide active.
- [ ] **SOIL-NUTR-005**: When the soil-test deficiency evaluator reports iron as Low, its suggestion shall state only the amount of iron per 1,000 sq ft as a foliar application, no amount of ferrous sulfate (also a moss-control pesticide active) or any other pesticide-active material, and shall never call ferrous sulfate chelated iron.

## Soil Tests page

- [x] **SOIL-UI-001**: The web UI shall serve the Soil Tests page at `/soil-tests`, listed in navigation as "Soil Tests".
- [x] **SOIL-UI-002**: When the Soil Tests page loads, it shall list the stored tests and request the soil-test advice only when at least one test exists.
- [x] **SOIL-UI-003**: The Soil Tests page shall use one form for adding and editing a test, grouped as Core, Macronutrients (ppm), Micronutrients (ppm) and Other, with the test date defaulting to today's local date and empty numeric fields sent as absent.
- [x] **SOIL-UI-004**: If the owner saves the Soil Tests form with no pH, then the page shall show "pH is required" and send nothing.
- [x] **SOIL-UI-005**: When a Soil Tests form save succeeds, the page shall reset and close the form and reload the test list and the advice.
- [x] **SOIL-UI-006**: Where soil-test advice includes a pH recommendation, the Soil Tests page shall show a pH Adjustment card with the current pH, target pH, amendment, amount and explanation.
- [x] **SOIL-UI-007**: Where the soil-test N-P-K recommendation advises N, P₂O₅ or K₂O, the Soil Tests page shall show a Fertilizer Recommendation card with the ratio, the N, P₂O₅ and K₂O amounts, the phosphorus and potassium levels, the example analysis with its amount (omitted when it is "N/A"), the remaining annual nitrogen and the explanation.
- [x] **SOIL-UI-008**: Where soil-test advice lists deficiencies, the Soil Tests page shall show a table of nutrient, current ppm, threshold ppm, level and suggestion.
- [ ] **SOIL-UI-009**: Wherever the Soil Tests page shows a nutrient level, it shall show the level's word (Low, Adequate, High, Not tested) together with its color, never color alone.
- [x] **SOIL-UI-010**: The Soil Tests page shall list stored tests newest first in a history table of date, lab, pH, P, K, Ca, Mg and OM %, each row with Edit (opens the form filled with that test) and Delete.
- [x] **SOIL-UI-011**: While no soil test is stored, the Soil Tests page shall show "No soil tests recorded yet. Add your first test above." and no advice.
- [ ] **SOIL-UI-012**: The Soil Tests page shall show the date of the test its advice is based on alongside the advice.
- [x] **SOIL-UI-013**: If a Soil Tests page request fails, then the page shall show a plain-language message saying what went wrong and what to do (no lawn profile: set one up in Settings; timeout: try again; otherwise the server's message), never a status code or response body.
- [ ] **SOIL-UI-014**: Wherever the Soil Tests page shows soil-test advice, it shall show the latest test's age (the time from its `test_date` to today's local date) beside the test date.
- [ ] **SOIL-UI-015**: While the latest soil test's `test_date` is more than 3 years before today's local date, the Soil Tests page shall suggest retesting alongside the advice.
- [ ] **SOIL-UI-016**: When the soil-test N-P-K recommendation advises no N, P₂O₅ or K₂O, the Soil Tests page shall show "No fertilizer needed" with the explanation and the remaining annual nitrogen, and no ratio, zero amounts or "N/A" analysis.
- [ ] **SOIL-UI-017**: When the owner clicks Delete on a Soil Tests history row, the page shall ask for confirmation naming the test's date and delete the test only once the owner confirms.
- [ ] **SOIL-UI-018**: The Soil Tests page shall title the deficiency table, which covers calcium, magnesium and sulfur as well as the micronutrients, "Secondary & micronutrients".

## Rejoining the feed

- [D] **SOIL-REJOIN-001**: Where soil-test advice is a recommendation-feed source, the system shall add it ahead of the nitrogen guard, the inventory pass and answered state, as `soil_test_ph` (Warning when the pH gap exceeds 1.0, else Advisory), `soil_test_npk` (Advisory, only when N, P₂O₅ or K₂O is advised) and `soil_test_micro_<nutrient>` (Info, one per Low nutrient), all in category SoilTest.
- [D] **SOIL-REJOIN-002**: Where soil-test advice is a recommendation-feed source, the system shall mark `soil_test_npk` as a feeding (for the nitrogen guard) only when it advises nitrogen above 0; a recommendation advising only P₂O₅ or K₂O shall not be a feeding.
- [D] **SOIL-REJOIN-003**: Where soil-test advice is a recommendation-feed source, while the nitrogen guard holds feedings (fertilizer block or Warning+ heat stress), the system shall hold the whole `soil_test_npk` recommendation, its P₂O₅ and K₂O included.
- [D] **SOIL-REJOIN-004**: Where soil-test advice is a recommendation-feed source, when the annual nitrogen budget is reached or limits the amount, the nitrogen guard shall cap only the nitrogen part of `soil_test_npk`, and its P₂O₅ and K₂O advice, severity and product need shall stand.
- [D] **SOIL-REJOIN-005**: Where soil-test advice is a recommendation-feed source, `soil_test_npk` shall add no "N budget remaining" data point of its own (the nitrogen guard writes it).
- [D] **SOIL-REJOIN-006**: Where soil-test advice is a recommendation-feed source, the system shall declare product needs as: lime → SoilAmendment of kind calcitic or dolomitic lime; sulfur → SoilAmendment of kind sulfur; N-P-K → Fertilizer; calcium → SoilAmendment of kind gypsum; sulfur deficiency → SoilAmendment of kind sulfur; iron, manganese, zinc, boron, copper → Supplement targeting that nutrient.
- [D] **SOIL-REJOIN-007**: Where soil-test advice is a recommendation-feed source, the system shall declare the low-magnesium product need as satisfied by either a Supplement targeting Magnesium or a SoilAmendment of kind dolomitic lime (an either-or need, matched by Product inventory).
