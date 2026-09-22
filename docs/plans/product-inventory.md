# Plan: Product Inventory (what's on the shelf, and does a recommendation need it)

**Status: agreed 2026-09-22 — not started.** Decisions recorded at the end; PR 1 can begin.

## Goal

Catalog the products on hand (fungicides, herbicides, insect control, fertilizer, nutrient
supplements, soil amendments, seed, surfactants) and thread that inventory through the app so that:

1. the Inventory page answers "what do I have and is it the right thing for X";
2. every recommendation that implies a product says whether it is **on hand**, **running low**, or
   **needs buying**, and which shelf product satisfies it;
3. logging an application picks the product from the shelf and pre-fills NPK / FRAC / rate;
4. entry is AI-assisted like landscape plants: type a product name, get a structured profile
   (category, actives, NPK, FRAC classes, what it targets, a suggested label rate) that the user
   confirms.

**Deliberately out of scope (owner, 2026-09-22): quantities.** No amounts, units, ledger, or
deduction on application. Stock is a hand-set status (In stock / Low / Out). Quantities can be
added later as a column plus a movements table without changing anything else here.
Also out: pricing, purchase history, barcode scanning, expiry.

## What exists today (the hooks)

| Piece | Where | Relevance |
|---|---|---|
| AI-assisted record creation (strict JSON schema, prompt cached, provenance columns, 503 without key) | `datasources/openrouter.rs`, `models/plant.rs`, `api/plants.rs`, `db/plant_queries.rs`, `pages/Landscape.tsx` | Pattern to clone for products |
| `applications.product_name` free text; `frac_classes TEXT[]`; NPK % columns; `rate_per_1000sqft` | `models/application.rs`, `pages/Applications.tsx:494` | Where the inventory picker goes |
| `FracClass` enum, `common_products()`, `PRODUCT_PATTERNS` → `frac_classes_for_product()` | `models/frac_class.rs` | The only curated product knowledge; stays the authority over the LLM for FRAC |
| `FungicideOption { frac_class, efficacy, examples, last_used, recommended }` and `recommended_class()` (best efficacy, unrestricted, not last used) | `models/disease_management.rs`, `logic/disease/management/mod.rs:119` | Lowest-friction "do I own a product in an eligible class" hook |
| `Recommendation` has no structured "what to apply" — products live in `description` / `suggested_action` prose | `models/recommendation.rs` | Needs a structured need field before inventory can match reliably |
| `nitrogen_guard::apply` mutates the combined recs after the rules run | `logic/rules/nitrogen_guard.rs`, `api/recommendation_feed.rs:45` | Precedent for a post-pass; inventory enrichment slots in at `recommendation_feed.rs:78` (after every source, before persisted answers are applied) |
| Soil-test recs already name amendments (lime/sulfur/gypsum/chelated iron…) and NPK ratios | `logic/soil_test_recommendations.rs` | Product-shaped, maps cleanly to categories |
| `PlannedActivity.product_suggestions` hard-coded strings | `logic/seasonal_plan.rs:200` | Could be replaced by inventory matches later (not in scope) |
| `docs/plans/ai-assistant.md` kill-criterion #3: "ground product naming in a real product DB or strip it" | | This inventory is that grounding layer |

## Domain model

### `Product` (new table `products`)

```
id, lawn_profile_id (FK, cascade)
name            TEXT NOT NULL             -- as the user knows it: "Heritage G", "Scotts Turf Builder 32-0-4"
brand           TEXT NULL
category        TEXT NOT NULL CHECK (...)  -- ProductCategory, see below
form            TEXT NOT NULL CHECK (...)  -- Granular | Liquid | WaterSoluble | Seed | Other
stock_status    TEXT NOT NULL DEFAULT 'InStock' CHECK (InStock | Low | Out)   -- hand-set, no counts
label_rate_per_1000sqft DOUBLE PRECISION NULL, label_rate_unit TEXT NULL     -- suggested by the LLM, confirmed by the user, optional
nitrogen_pct, phosphorus_pct, potassium_pct DOUBLE PRECISION NULL           -- user-confirmed (AI pre-fills)
frac_classes    TEXT[] NULL               -- user-confirmed (curated resolver > AI)
herbicide_timing TEXT NULL CHECK (PreEmergent | PostEmergent | Both)
targets         TEXT[] NULL               -- controlled vocabulary, see ProductTarget
amendment_kind  TEXT NULL CHECK (CalciticLime | DolomiticLime | Sulfur | Gypsum | Humic | Compost | Other)
profile         JSONB NULL                -- ProductProfile from the LLM; NULL = entered by hand
profile_generated_at TIMESTAMPTZ NULL, profile_model TEXT NULL
notes TEXT NULL, archived BOOLEAN NOT NULL DEFAULT false
created_at, updated_at
```

The columns above `profile` are the **matchable** facts and are what the recommendation pass
reads. They are copied out of the AI profile on save, then owned by the user (editable on the
page). The profile JSON is informational (summary, actives, cautions) and regenerable without
touching the user's corrections — the same split as `update_plant_metadata` vs `update_plant_plan`.

`Out` is distinct from `archived`: Out means "I use this and need to rebuy" (feeds the shopping
list); archived means "I no longer use this" (hidden everywhere).

### `ProductCategory`

| Variant | Covers | Matches `ApplicationType` |
|---|---|---|
| `Fertilizer` | granular / liquid N-P-K, organics (Milorganite), starter fert | `Fertilizer`, `PlantFertilizer` |
| `Supplement` | iron, micronutrients, humic/kelp/biostimulants ("nutrient supplements") | `Fertilizer`, `PlantFertilizer`, `Other` |
| `Fungicide` | anything with a FRAC class | `Fungicide` |
| `Herbicide` | pre-emergent, post-emergent, non-selective; `herbicide_timing` says which | `PreEmergent`, `PostEmergent` |
| `InsectControl` | grub preventatives/curatives, surface insects (ants, ticks, chinch bug), termite, repellents | `GrubControl`, `Insecticide` |
| `Seed` | grass seed; `targets` carries the species mix (KBG / TTTF / PRG / fine fescue) | `Overseed` |
| `SoilAmendment` | lime, sulfur, gypsum, compost, humic; `amendment_kind` says which | `Lime`, `Sulfur`, `Other` |
| `Surfactant` | wetting agents, spreader-stickers, adjuvants | `Wetting` |
| `Other` | | any |

`ProductTarget` (controlled vocabulary, `TEXT[]`): `Grubs, SurfaceInsects, Ants, Termites, Ticks,
Mosquitoes, Broadleaf, Crabgrass, Nutsedge, Poa, Moss, Iron, Manganese, Magnesium, Zinc, Boron,
Copper, Calcium, Kbg, Ttf, Prg, FineFescue, Bermuda, Zoysia`. Free text would make matching
impossible; the LLM schema enumerates exactly this list.

### `ProductProfile` (LLM output, strict JSON schema, stored as JSONB)

```
identified_name, manufacturer, category, form,
active_ingredients: [{ name, pct? }],
npk: { n, p, k }?,  frac_classes: [FracClass]?,  herbicide_timing?,  targets: [ProductTarget],
amendment_kind?,  mobility: Systemic | Contact | NotApplicable,
nitrogen_release: Quick | Slow | Mixed | NotApplicable,
suitable_for: [Turf, Ornamentals, Both],
suggested_label_rate: { amount, unit: Lb | Oz | FlOz, per_1000sqft: true }?   -- null unless commonly known
summary, cautions: [string], confidence: High | Medium | Low
```

**The rate is the one place this feature departs from the app's "no rates — label governs" rule
(owner decision 3).** Containment: the prompt says "only if the product's label rate is commonly
published; otherwise null; never estimate"; the value is stored as a *suggestion* the user confirms
or clears; it is shown on the product row as "Suggested rate — verify against your label" and used
only to pre-fill the application form's rate field. It is never written into any recommendation
text, disease program, or timing window. No interval, no "apply when" in the schema.

## Linking to applications (owner: yes)

- `applications.product_id BIGINT NULL REFERENCES products(id) ON DELETE SET NULL`.
  `product_name` stays and is copied from the product at log time, so history survives a rename
  or delete and the existing duplicate key on `(type, date, product_name)` is untouched.
- **Form** (`Applications.tsx:494`, extracted into a new `components/ProductPicker.tsx` — the page
  is already 820 lines): a select listing non-archived products whose category is compatible with
  the chosen `ApplicationType` (table above), sorted by name, plus "Not in inventory…" which reveals
  today's free-text input and an "Add to inventory" link that opens the Inventory add form
  pre-filled. Picking a product pre-fills NPK, FRAC classes (bypassing the name lookup in
  `FracClassPicker` — the product's confirmed classes win), and rate (from the suggested label rate,
  if any); every field stays editable.
- Logging an application against a product that is `Out` prompts "mark as In stock?" (one click)
  — the only stock-status automation.
- **Backfill**: `GET /inventory/suggestions` returns distinct `product_name`s from turf/plant
  applications that match no product (case-insensitive), with counts and last date, so the
  Inventory page can offer "You've logged *Heritage G* 3× — add it?" (one click → AI-assisted
  add with the name pre-filled). A "link past applications by name" button on each product sets
  `product_id` on matching rows on request.
- Nitrogen budget, rotation analysis and disease history keep reading the application row; nothing
  there changes.

## Dovetailing into recommendations

### 1. Structured needs on `Recommendation`

```rust
pub struct ProductNeed {
    pub category: ProductCategory,
    pub label: String,                        // "pre-emergent herbicide", "fungicide (FRAC 3, 7 or 11)"
    pub frac_classes: Vec<FracClass>,         // any-of; empty = no constraint
    pub herbicide_timing: Option<HerbicideTiming>,
    pub targets: Vec<ProductTarget>,          // any-of
    pub amendment_kind: Option<AmendmentKind>,
}
// on Recommendation:
#[serde(default)] pub needs: Vec<ProductNeed>,
#[serde(default, skip_serializing_if = "Option::is_none")] pub inventory: Option<InventoryStatus>,
```

`with_need(ProductNeed)` builder. Both fields default empty, so every existing rule, test and the
persisted state model keep working untouched; sources adopt needs one at a time.

Which sources declare what:

| Source | Need |
|---|---|
| `grub_control_*` (preventative) / `grub_control_late_*` (curative) | `InsectControl`, targets `[Grubs]`, label distinguishes preventative vs curative |
| `broadleaf_spring` / `broadleaf_fall` | `Herbicide`, timing `PostEmergent`, targets `[Broadleaf]` |
| `timing_spring_pre_emergent_*`, `timing_fall_pre_emergent_*` | `Herbicide`, timing `PreEmergent` |
| `timing_fall_seeding_*`, `timing_spring_seeding_*`, dormant | `Seed`, targets from `GrassType` (Mixed → any cool-season species) |
| `fall_fert_*`, `spring_n_ready` (after `nitrogen_guard`) | `Fertilizer` |
| `soil_test_ph` | `SoilAmendment`, kind lime (calcitic or dolomitic) or sulfur |
| `soil_test_npk` | `Fertilizer` (P/K side); a second need `Supplement` when only P or K is short |
| `soil_test_micro_<x>` | `Supplement`, targets `[Iron]` etc.; gypsum/lime cases → `SoilAmendment` |
| `disease_<slug>` | `Fungicide`, `frac_classes` = the program's unrestricted, eligible-at-this-tier classes minus the last-used class (from `recommended_class`'s candidate set); red thread → `Fertilizer` need instead (its remedy is N) |
| `irrigation_forecast_*` under drought | optional `Surfactant` need, label "wetting agent (optional, for hydrophobic spots)" — Info-only, never changes severity |
| plant `Fertilizing` tasks | `Fertilizer` or `Supplement`, `suitable_for` Ornamentals |

### 2. Enrichment pass `logic/inventory/enrich.rs`

Pure function `enrich(recs: &mut [Recommendation], products: &[Product])`, called in
`recommendation_feed::all` between the soil-test source and the persisted-answer step. For each
rec with needs:

- match products: category equal, not archived, status ≠ `Out`; for fungicide needs any product
  class ∈ need classes; for herbicide the timing matches or is `Both`; targets intersect when the
  need lists any; amendment kind equal.
- `InventoryStatus { state: OnHand | Low | NotOnHand | Partial, matches: Vec<InventoryMatch>, missing: Vec<String> }`
  where `InventoryMatch { product_id, name, stock_status }`. `Low` when every match is Low;
  `Partial` when some needs are covered and others not (soil-test NPK with P covered, K not);
  `missing` carries the labels of unmet needs (the shopping-list text).
- Never changes `id`, `severity` or `title` (persisted answers key on id + severity). Appends one
  `DataPoint("On hand", "Heritage G (FRAC 11)", "Inventory")` per match so the existing detail
  table shows it with no UI change, and sets `inventory` for the new UI.

### 3. Disease management: prefer what is on the shelf

- `FungicideOption` gains `on_hand: Vec<InventoryMatch>`.
- `recommended_class()` ordering becomes: efficacy → not restricted → not last-used → **on hand**
  → spec order. Inventory is the last tiebreak: it never promotes a lower-efficacy class and never
  overrides rotation (owning a bag of the last-used class does not make it the recommendation).
- The headline/`suggested_action` in `disease::recommendations` gets "…you have Heritage G
  (FRAC 11) on hand" or "…nothing on hand in FRAC 3 / 7 / 11" appended.
- `ManagementPanel.tsx` shows a shelf pill on each option.

### 4. Shopping list

- `GET /api/v1/inventory/shopping-list`: the `missing` needs of active recs, products marked `Out`
  or `Low` whose category is needed by an active rec **or** by a Seasonal Plan activity in the next
  45 days, each with the reason ("Fall feeding #2: fertilizer"; "Brown patch High: FRAC 3 or 7";
  "Heritage G is marked Low"). A Low bag of something with no upcoming use is not listed.
- Rendered at the top of the Inventory page and as a compact dashboard widget (`InventoryWidget`).
- No new recommendation category: the shopping list is a view, not an alert. (Revisit if the
  dashboard widget turns out not to be enough.)

## API

| Method | Path | Notes |
|---|---|---|
| GET | `/api/v1/products` | `?category=&include_archived=` |
| POST | `/api/v1/products` | body: name (+ optional brand, category, form, status, the matchable facts). When OpenRouter is configured and `assist: true` (default), calls it and returns 201 with the profile; when unconfigured or `assist: false`, saves with `profile = NULL` (owner decision 1) |
| GET/PUT/DELETE | `/api/v1/products/:id` | PUT = user-owned fields (metadata, matchable facts, status, rate); never touches `profile` |
| POST | `/api/v1/products/:id/refresh-profile` | (re)generate; re-run the curated FRAC resolver; **do not** overwrite user-confirmed fields — return the profile plus a `suggestions` diff the UI shows as "profile suggests…" with per-field Apply |
| POST | `/api/v1/products/:id/link-applications` | set `product_id` on past applications whose `product_name` matches by name |
| GET | `/api/v1/inventory/suggestions` | unmatched `product_name`s from the log |
| GET | `/api/v1/inventory/shopping-list` | derived, PR 4 |

OpenRouter: new `generate_product_profile(ProductProfileRequest { name, brand, category_hint,
grass_type, usda_zone })` beside `generate_plant_plan`, same client, same mandatory
`HTTP-Referer` / `X-Title` headers, `cache_control` on the system prompt, strict schema with
`additionalProperties: false`, enums mirrored 1:1 from the Rust types, `temperature 0.1`. Model
via the existing `OPENROUTER_MODEL` (no new env var). Unlike plants, an LLM failure on POST does
not fail the request: the product is saved by hand and the response carries `profile_error` so
the page can offer Regenerate.

### Hallucination guardrails (products are worse than plants here)

- FRAC classes: `frac_classes_for_product(name)` runs first; if it returns anything, it wins and
  the LLM's list is only shown as "also suggested" when it differs. `PRODUCT_PATTERNS` grows as
  the user's real products get added (a small PR each time, no LLM involved).
- The profile carries `confidence`; Low/Medium renders a "verify against the label" banner on the
  product and the matchable facts stay editable in place.
- The suggested rate is fenced as described above; `cautions` is limited to label-class facts
  (e.g. "not labeled for residential use", "do not apply above 85 °F").
- Every AI-derived field is copied into user-owned columns on save; the LLM never writes to a
  column after that except via an explicit refresh, which returns suggestions rather than
  overwriting.

## Frontend

- `pages/Inventory.tsx` (nav "Inventory" after Applications; route in `App.tsx`): grouped by
  category with a stock pill (In stock / Low / Out) using the status palette + symbol + label as
  elsewhere, and a one-click status cycle on the row; row expands to the profile (summary, actives,
  cautions, suggested rate with its caveat, provenance line `Profile generated … · model` or
  `Entered by hand`), Edit, Regenerate profile, Link past applications, Archive. Top: "Needs
  buying" (PR 4) and "From your log" suggestions.
- Add form mirrors the plant form: one required free-text name, optional brand, category select
  defaulting to "Let the assistant decide" (required when the assistant is off), form, status.
  Button flips to "Identifying…", with the 10–30 s hint; 60 s client timeout as for plants. On
  success the new row auto-expands with the AI facts highlighted for confirmation.
- `components/ProductPicker.tsx` for the application form (above).
- `components/InventoryBadge.tsx`: one chip — ✓ On hand · ⚠ Low · ✗ Need to buy · ◐ Partial —
  used by `AlertCard` (dashboard) and the Recommendations list; the detail pane gets an
  "Inventory" section listing matches.
- `types/inventory.ts` mirrors the Rust types; label maps for categories, targets, statuses.

## Delivery (stacked PRs, each green on `cargo test` / `clippy` / `tsc` / biome)

1. **Catalog** — migration (`products`), `models/product.rs`, `db/product_queries.rs`,
   OpenRouter profile method + schema, `api/products.rs` (+ `products/refresh.rs` if it passes
   300 lines), Inventory page split into page + `components/inventory/{ProductRow,AddProductForm}.tsx`,
   nav, types. Tests: schema-shape, curated-resolver-wins, manual save without OpenRouter, query
   round trip. `CLAUDE.md` + README endpoint tables.
2. **Applications link** — `product_id` migration, `ProductPicker`, pre-fill, "Out → In stock?"
   prompt, suggestions + link-applications endpoints. Tests: compatibility map, name matching.
3. **Recommendations** — `ProductNeed` / `InventoryStatus`, needs declared by each source in the
   table, `enrich` pass, disease `on_hand` + tiebreak, `InventoryBadge`, detail section. Tests:
   one per source's need, enrich matching matrix, rotation beats inventory.
4. **Shopping list** — endpoint, Inventory page section, dashboard widget.

Rough size: PR 1 ≈ 1,100 lines, PR 2 ≈ 400, PR 3 ≈ 800, PR 4 ≈ 300.

## Decisions (owner, 2026-09-22)

1. **Save without the LLM: yes.** Manual entry is first-class; the profile is optional and can be
   generated later. (Differs from plants, which refuse to save without OpenRouter.)
2. **No quantity tracking.** No amounts, units, ledger or deduction. Stock is a hand-set
   In stock / Low / Out. Revisit only if the status toggle proves insufficient.
3. **Suggest the label rate, don't require it.** Stored as an optional, user-confirmable
   suggestion, fenced as described under `ProductProfile`.
4. **Quantity model: none** (follows from 2).
5. **Categories as tabled**, with insect control (grubs, ants, termites, ticks) its own category.
