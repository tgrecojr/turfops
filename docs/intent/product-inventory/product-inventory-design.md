---
parent: high-level-design
prefix: INV
---

# Product Inventory (the Shelf)

## Context and Design Philosophy

The shelf is the owner's catalog of lawn-care products on hand: fertilizer, nutrient supplements, fungicides, herbicides, insect control, seed, soil amendments and surfactants. It exists so that every recommendation can say whether the product it calls for is **on hand**, **running low**, or **needs buying**, and so the shopping list holds only what active advice requires (HLD goal "Recommendations reflect the shelf").

Guiding principles:

- **Facts are the owner's; the profile is the assistant's.** A product has user-owned, matchable *facts* (category, form, N-P-K, FRAC classes, herbicide timing, targets, amendment kind, optional label rate) and an optional LLM *profile* that is informational and regenerable. Matching reads only facts. The LLM fills facts once at add time; after that it only *suggests*.
- **Precedence: owner > curated > LLM.** The owner's category and form beat the LLM's guess; the curated FRAC resolver (`frac_classes_for_product`, owned by Applications) beats the LLM's FRAC list.
- **No quantities.** Stock is a hand-set In stock / Low / Out. `Out` means "I use this and need to rebuy"; `archived` means "I no longer use this" and hides the product everywhere.
- **Strict matching.** A product with no FRAC class, timing or target recorded does not satisfy a need that asks for one. A false "on hand" sends the owner to the shed for the wrong bag.
- **Saves without the LLM.** Manual entry is first-class; a profile can be generated later. An assistant failure or an implausible suggested value never loses the product: it saves by hand, minus the value it cannot store.
- **One name, one product.** A product's name is unique within the lawn (case-insensitive, trimmed), so a logged name links to one product and the shopping list never shows two rows with the same label.
- **Sources declare, the shelf answers.** Each recommendation source in the feed (Agronomic rules, Timing windows, Disease risk, Landscape plants) owns the `ProductNeed`s it declares. Soil-test advice is not in the feed, so no soil-test need reaches the inventory pass or the shopping list. This component owns the need vocabulary, the matching, the feed's inventory pass, the shopping list, and the shelf itself.

## Domain Model

`Product` (`backend/src/models/product.rs`) carries `name`, `brand`, `stock_status`, flattened `facts: ProductFacts`, `profile: Option<ProductProfile>` with provenance (`profile_generated_at`, `profile_model`), `notes`, `archived`, timestamps. The wire format is flat: facts serialize at the top level (`product_json_is_flat` test).

### Controlled vocabularies

All enums are declared with the `string_enum!` macro (`backend/src/models/product/enums.rs:7`). Each variant's name is simultaneously its serde name, its database string and its JSON-schema enum value, so `as_str()`, `FromStr` (case-insensitive, trimmed) and serde agree by construction. `ALL` lists every variant and feeds the LLM schema.

| Enum | Variants |
|---|---|
| `ProductCategory` | Fertilizer · Supplement · Fungicide · Herbicide · InsectControl · Seed · SoilAmendment · Surfactant · Other |
| `ProductForm` | Granular · Liquid · WaterSoluble · Seed · Other |
| `StockStatus` | InStock · Low · Out |
| `HerbicideTiming` | PreEmergent · PostEmergent · Both |
| `AmendmentKind` | CalciticLime · DolomiticLime · Sulfur · Gypsum · Humic · Compost · Other |
| `RateUnit` | Lb · Oz · FlOz (per 1,000 sq ft) |
| `Mobility` / `NitrogenRelease` / `SuitableFor` | profile-only descriptors |
| `ProductTarget` | pests (Grubs, SurfaceInsects, Ants, Termites, Ticks, Mosquitoes) · weeds (Broadleaf, Crabgrass, Nutsedge, Poa, Moss) · nutrients (Iron, Manganese, Magnesium, Zinc, Boron, Copper, Calcium) · species (Kbg, Ttf, Prg, FineFescue, Bermuda, Zoysia) |

### Category ↔ application type (`ProductCategory::accepts`)

`accepts(ApplicationType)` (`enums.rs:68`) decides which shelf products can be logged under an application type. `Other` on either side accepts anything.

| Category | Accepts |
|---|---|
| Fertilizer, Supplement | Fertilizer, PlantFertilizer |
| Fungicide | Fungicide |
| Herbicide | PreEmergent, PostEmergent |
| InsectControl | GrubControl, Insecticide |
| Seed | Overseed |
| SoilAmendment | Lime, Sulfur |
| Surfactant | Wetting |
| Other | any |
| (any category) | application type Other |

The frontend mirrors this by hand in `categoriesFor` (`frontend/src/types/inventoryCompat.ts:10`), which always appends `Other`. `guessCategory` picks the first non-`Other` category for a logged name's application types (used to pre-fill the add form from the log).

### Scoping facts to the category (`ProductFacts::scoped_to_category`)

`backend/src/models/product/profile.rs:118`. Facts that cannot apply to a category are dropped:

| Fact | Kept only on |
|---|---|
| `frac_classes` | Fungicide, Other |
| `herbicide_timing` | Herbicide, Other |
| `amendment_kind` | SoilAmendment, Other |
| `targets` | the family `ProductTarget::fits_category` allows (`enums.rs:138`): pests → InsectControl; weeds → Herbicide; nutrients → Fertilizer and Supplement; species → Seed; none on Fungicide, SoilAmendment, Surfactant; anything on Other |

N-P-K and the label rate are not scoped. Scoping runs on add, on refresh (for the suggested facts) and on every PUT, so stored facts are always category-consistent. The data migration `20260923000000_product_facts_scoped.sql` applies the same rules to stored rows.

## Adding, Editing and Refreshing a Product

### Add (`POST /api/v1/products`, `backend/src/api/products.rs:73`)

1. Trim the name; empty → 400. A name that matches another of the lawn's products (case-insensitive, trimmed, archived included) → 409 with a plain-language message, checked before the assistant is asked, nothing saved (INV-API-013).
2. If `assist` (default true) **and** OpenRouter is configured, ask for a profile using `lookup_name(brand, name)` (brand prefixed unless the name already contains it, `product.rs:38`), the lawn's grass type, USDA zone, and the owner's category as a hint.
   - Success → facts = `ProductFacts::from_profile(profile, frac_classes_for_product(name))`: the curated FRAC list wins when non-empty, else the LLM's; the suggested label rate becomes the stored label rate.
   - Out-of-range suggestions → a suggested label-rate amount ≤ 0 (amount and unit dropped together) or an N, P or K percentage outside 0–100 (only that percentage dropped) is left out of the facts with a logged warning, and the product saves with the rest (INV-LLM-010). The profile itself is stored as returned; it is informational.
   - Failure → logged as a warning; `profile_error` carries a plain-language message; continue by hand.
3. Without a profile, facts = `ProductFacts::manual(category, form or Other)` plus the curated FRAC classes for the name. The category is the owner's; when the owner gave none, the assistant was requested and it failed, the product saves as `Other` and `profile_error` tells the owner (INV-LLM-011). A missing category is a 400 only when no assistant was requested, or when it was requested but OpenRouter is absent ("assistant is not configured").
4. The owner's `category` and `form`, when given, overwrite the LLM's.
5. `scoped_to_category()`.
6. Brand = the owner's brand, else the profile's manufacturer. Stock defaults to InStock.
7. Insert and return `201 { product, profile_error }`.

### Edit (`PUT /api/v1/products/:id`, `products.rs:181`)

Replaces every user-owned field (name, brand, stock status, facts, notes, archived) and never touches the profile. Validation: non-empty name; label rate amount and unit both present or both absent; amount above 0 and each of N, P and K within 0–100, each failure a 400 naming the field and its range (INV-API-011); a name already used by another of the lawn's products → 409 (INV-API-013). Facts are scoped before saving. Blank brand and notes become NULL. Missing id → 404.

The Inventory page's stock-pill cycle and Archive buttons go through this same PUT, sending the full product from the page's copy (`frontend/src/components/inventory/useProductActions.ts:56`).

### Refresh (`POST /api/v1/products/:id/refresh-profile`, `backend/src/api/products/refresh.rs`)

Requires OpenRouter (503 otherwise). Generates a new profile with the product's current category as the hint, **stores the profile and its provenance**, and returns `{ product, suggested_facts, changed_fields }`. `suggested_facts` = `from_profile` with the curated FRAC resolver, scoped to the *suggested* category. `changed_fields` (`ProductFacts::changed_fields`, `profile.rs:136`) names the differing fields: `category`, `form`, `label_rate`, `npk`, `frac_classes`, `herbicide_timing`, `targets`, `amendment_kind`. The owner's facts are not changed; the page shows "The refreshed profile suggests changes to …" with "Review in editor" (opens the edit form pre-filled with the suggested facts) or "Ignore".

## LLM Profile Contract

`backend/src/datasources/openrouter/product.rs`, using the shared OpenRouter client (`backend/src/datasources/openrouter.rs`).

- **Request**: strict `json_schema` response format (`additionalProperties: false`, every property required, nullable fields as `anyOf [.., null]`), temperature 0.1, system prompt sent with `cache_control: ephemeral`, model from `OPENROUTER_MODEL`. 60 s request timeout, 5 s connect timeout.
- **Attribution**: every request carries `HTTP-Referer: https://github.com/tgrecojr/turfops` and `X-Title: TurfOps` (`openrouter.rs:124`), so per-app usage is visible on the OpenRouter console.
- **Schema**: `identified_name, manufacturer, category, form, active_ingredients[{name, pct}], npk{n,p,k}?, frac_classes[], herbicide_timing?, targets[], amendment_kind?, mobility, nitrogen_release, suitable_for, suggested_label_rate{amount, unit}?, summary, cautions[], confidence`. Enum values come from each Rust enum's `ALL`; FRAC values from the module's `ALL_FRAC` list. There are no interval or application-timing fields (asserted by `schema_requires_every_profile_field_and_has_no_rate_interval`).
- **Prompt rules** (`PRODUCT_SYSTEM_PROMPT`): identify the exact retail product, lower confidence when ambiguous, return Low confidence and `Other` (or the owner's category) when unrecognized, never invent a formulation; report only label facts; FRAC codes only from the list, empty when unsure; `suggested_label_rate` only when the label rate is commonly published and the model is confident, otherwise null, never estimated; at most 4 label-class cautions; a 1–2 sentence summary tuned to grass type and zone.

### The product label-rate rule

This is the HLD's label-rate rule, applied to the one place a product's label rate lives. Advice never states a product's label rate; the label governs how much of a product to apply. A label rate the owner records, optionally suggested by the LLM, is:

- **stored** as the product's `label_rate_per_1000sqft` + `label_rate_unit` (a fact the owner can edit or clear);
- **shown with its caveat** on the product: "suggested by the assistant; verify against your label. Pre-fills the application form only." (or "from your label" once the owner has changed it, `ProductDetails.tsx:120`);
- **used only to pre-fill** the application form's rate (Applications owns the form);
- **never in advice**: not in recommendation text, disease programs, timing windows or the shopping list;
- **reported on request through MCP**: when the owner asks an agent about that product, the agent may state the recorded rate as the owner's value with "verify against your label" (the MCP server owns that wording).

Agronomic amounts (nutrient per 1,000 sq ft, lime or sulfur from a soil test, seed per 1,000 sq ft) are not label rates; they belong to the sources that state them, and nothing on the shelf converts them into an amount of a specific product.

## Persistence

`products` (`backend/src/db/migrations/20260922000000_products.sql`), FK `lawn_profile_id → lawn_profiles ON DELETE CASCADE`, index on `lawn_profile_id`. `frac_classes` and `targets` are `TEXT[] NOT NULL DEFAULT '{}'`; `profile` is JSONB. CHECK constraints: category, form, stock status, rate unit, herbicide timing, amendment kind within their vocabularies; rate amount and unit both NULL or both set, amount > 0; N-P-K each 0–100; profile, `profile_generated_at` and `profile_model` all NULL or all set. Product names are unique within a lawn, compared case-insensitively after trimming (INV-API-013). The handlers validate every CHECKed range before writing, so a constraint violation never reaches the owner as a 500.

`applications.product_id BIGINT NULL REFERENCES products(id) ON DELETE SET NULL` (`20260922000100_application_product.sql`), indexed. The application keeps its own `product_name` copy, so history survives a rename or delete and the application duplicate key is unchanged.

Reads (`backend/src/db/product_queries.rs`): list by lawn profile, optional category, archived excluded unless asked, ordered by category then `lower(name)`. Unknown stored enum strings in optional or list columns are dropped with a warning (`parse_enum`, line 180); an unknown category or form fails the row with 400; an unknown stock status reads as InStock; invalid profile JSON fails the row.

## API Endpoints

| Method | Path | Handler | Response / errors |
|---|---|---|---|
| GET | `/api/v1/products?category=&include_archived=` | `list_products` | `Product[]` |
| POST | `/api/v1/products` | `create_product` | 201 `{product, profile_error}`; 400 empty name / missing category with no assistant; 409 name already on the shelf |
| GET | `/api/v1/products/:id` | `get_product` | `Product`; 404 |
| PUT | `/api/v1/products/:id` | `update_product` | `Product`; 400 name / unpaired or out-of-range rate / N-P-K out of range; 404; 409 name already on the shelf |
| DELETE | `/api/v1/products/:id` | `delete_product` | 204; 404 unknown id |
| POST | `/api/v1/products/:id/refresh-profile` | `refresh_product_profile` | `{product, suggested_facts, changed_fields}`; 503 no OpenRouter or LLM failure; 404 |
| POST | `/api/v1/products/:id/link-applications` | `link_applications` | `{linked}` |
| GET | `/api/v1/inventory/suggestions` | `logged_product_suggestions` | `[{name, uses, last_used, application_types}]` |
| GET | `/api/v1/inventory/shopping-list` | `get_shopping_list` (`backend/src/api/inventory.rs`) | `{generated_at, items[]}` |

The MCP server's `products` and `shopping_list` tools call these handlers directly (MCP server owns the tools).

## Log ↔ Shelf Bridge

- **Suggestions** (`backend/src/db/product_queries/link.rs:17`): application rows with no `product_id` and a non-blank `product_name` that matches no product of the lawn (case-insensitive, trimmed, archived products included), grouped by lowercased name. Each row reports the most recent spelling, use count, last date and distinct application types; ordered by uses then recency. The page's "From your application log" list offers "Add to shelf", which opens the add form pre-filled with the name and `guessCategory`.
- **Link past applications** (`link.rs:47`): sets `product_id` on this lawn's unlinked rows whose `product_name` equals the product's name (case-insensitive, trimmed); returns the count. Already-linked rows are never reassigned.
- **Contract with Applications**: Applications validates a submitted `product_id` with `ProductCategory::accepts` and copies `product_name` from the product when the request has none (`backend/src/api/applications/product_link.rs`). The application form's product picker and its "mark In stock?" offer after logging against an `Out` product belong to Applications.

## Needs and Matching

`backend/src/models/inventory.rs`.

`ProductNeed { label, categories[], frac_classes[], herbicide_timing?, targets[], amendment_kinds[], optional, requires_nitrogen }`, built with `new(label, category)` and `or_category`, `with_frac_classes`, `with_timing`, `with_targets`, `optional`, `requiring_nitrogen`. `amendment_kinds` has no builder because no feed source declares one; it is kept for amendment needs (soil-test advice, if it rejoins the feed). `optional` and `requires_nitrogen` default to false when absent from JSON (`frontend/src/types/inventory.ts` mirrors both). Sources attach needs with `Recommendation::with_need`. `label` is the plain-words text the UI and shopping list show.

**`fits(product)`** (facts only, stock ignored) is the conjunction of:

- category ∈ `categories` (an empty list matches nothing);
- `frac_classes` empty, or any shared with the product;
- `targets` empty, or any shared with the product;
- `herbicide_timing` absent, or the product's timing equals it or is `Both` (a product with no timing fails);
- `amendment_kinds` empty, or the product's kind is among them (a product with no kind fails);
- when the need requires nitrogen (`requires_nitrogen`: the nitrogen-fertilizer need every feeding declares, from the feeding rules and red thread), the product's N is above 0 or unrecorded. Unrecorded N matches here, unlike the other axes, because most fertilizers carry nitrogen and the owner often records a fertilizer without its analysis; a recorded 0-0-x is the case to exclude.

Each axis is any-of; axes are all-of. **`matches(product)`** = `fits` and not archived and stock ≠ Out. Low counts as matching.

A need can also be a set of **alternatives**, any one of which satisfies it, each alternative carrying its own categories and constrained axes (INV-NEED-006, deferred). This is the shape for a need that different categories meet through different facts: the soil-test magnesium need is met by a Supplement with a Magnesium target or by dolomitic lime, and no single all-of need can express that because scoping keeps targets off SoilAmendment and amendment kinds off Supplement. It is needed once soil-test advice rejoins the feed.

### InventoryStatus

`InventoryStatus::assess(needs, products)` (`inventory.rs:163`) returns `Option<{ state, matches[], missing[] }>`. `matches` holds one `InventoryMatch { need, product_id, name, stock_status }` per (need, matching product); `missing` lists the labels of required needs nothing matches. Optional needs contribute matches but are never counted or listed missing.

| State | Condition (evaluated in order) |
|---|---|
| — (None) | no required needs and no matches |
| NotOnHand | ≥1 required need, none covered |
| Partial | some but not all required needs covered |
| Low | at least one product matches a required need, and every product matching a required need is Low |
| OnHand | otherwise |

"Running low" is judged only from the products serving required needs (INV-ENRICH-002): an in-stock nice-to-have does not hide that the bag the work needs is running low, and a Low nice-to-have does not make covered work read as running low.

### Enrichment pass (`backend/src/logic/inventory.rs`)

`enrich(recs, products)` runs inside the Recommendation feed after every source and after the nitrogen guard has covered every feeding recommendation, and before persisted answers are applied (`recommendation_feed::assemble`, `backend/src/api/recommendation_feed.rs:111`). The guard's contract with this pass: at "target reached" the guard withdraws the nitrogen part of a feeding, so a nitrogen-only feeding carries no Fertilizer need and neither shows "need to buy" nor shops. The feed passes the lawn's non-archived products; if that read fails, the feed logs a warning and skips this pass, so recommendations go out without inventory status rather than not at all (INV-ENRICH-005). For each recommendation with needs and a `Some` status, it appends one `DataPoint("On hand", "<product> (<need>)", "Inventory")` per match and one `DataPoint("Not on shelf", "<need>", "Inventory")` per missing need, and sets `rec.inventory`. It never changes `id`, `severity` or `title`, so answered-state matching is unaffected.

### Shelf view for Disease risk

`ShelfProduct { product_id, name, stock_status }` and `ShelfFungicide { product, classes }` are defined here. Disease risk fills `DiseaseContext.shelf` with the lawn's non-archived, non-Out Fungicide products and their FRAC classes (`backend/src/api/disease_risk.rs:82`) and uses it only to break ties between equally efficacious eligible classes and to name owned products. That logic belongs to Disease risk.

## Shopping List

`backend/src/logic/inventory/shopping.rs`, `backend/src/models/shopping.rs`. The handler refreshes the environmental summary, takes the **active** feed (`recommendation_feed::active`, answered recommendations removed) and the non-archived shelf, and calls `build`.

For each active recommendation and each **required** need:

- **Restock**: every non-archived product with stock Low or Out that `fits` the need becomes (or adds a reason to) a Restock item keyed by product id, labelled with the product name. This applies even when another in-stock product covers the need.
- **Buy**: when the need's label is in `rec.inventory.missing` **and** no Low/Out product fits it, a Buy item keyed by the need label. A restockable product *is* the thing to buy, so it suppresses the generic Buy line.

Each item collects the distinct titles of the recommendations calling for it (`reasons`, in the feed's order, most severe first) and `urgency` = the highest severity among them. Items sort by urgency descending, then Buy before Restock, then label. Optional needs never shop.

The active feed is the whole horizon: timing windows enter it as Info while "opening soon", so upcoming work is already represented.

## UI

| Element | File | Behavior |
|---|---|---|
| Inventory page (`/inventory`) | `frontend/src/pages/Inventory.tsx` | Loads all products (archived included), groups by category in `PRODUCT_CATEGORIES` order, sorts by name; "Show n archived" toggle; add form; notice when a product was saved by hand after a profile error; Shopping list panel and log suggestions above the list. |
| Product row | `components/inventory/ProductRow.tsx`, `ProductDetails.tsx` | Header: name, brand, form, N-P-K, FRAC codes, timing, amendment kind, targets; summary when collapsed. Expanded: details (confidence banner when not High, label rate with its caveat), provenance line ("Profile generated … · model" or "Entered by hand — no assistant profile"), Edit, Generate/Regenerate profile, Link past applications, Archive/Unarchive, Delete (browser confirm). |
| Stock pill | `components/inventory/StockBadge.tsx` | Click cycles In stock → Low → Out → In stock; symbol + label over a tinted status color. |
| Add form | `components/inventory/AddProductForm.tsx` | Name (required), brand, category ("Let the assistant decide" or required when the assistant is off), form, stock, notes, assistant checkbox; "Identifying…" with a 10–30 s hint; 60 s client timeout (`api/client.ts:349`). |
| Edit form | `components/inventory/EditProductForm.tsx`, `CategoryFields.tsx` | Category-specific fields: N-P-K for Fertilizer/Supplement, FRAC picker for Fungicide, timing for Herbicide, amendment kind for SoilAmendment, target checkboxes from `TARGETS_BY_CATEGORY` (`types/inventoryTargets.ts:32`), which offers exactly the target family the backend keeps for the category (INV-UI-007), so no checked target is silently dropped on save. |
| Log suggestions | `components/inventory/LogSuggestions.tsx` | Hidden when empty or on fetch failure; per-session dismiss. |
| Shopping list | `components/inventory/ShoppingListPanel.tsx` | Full on the Inventory page (re-keyed on a shelf version built from id, stock, archived and `updated_at`); compact on the dashboard: hidden when empty, loading or failed, top 4 + "n more". Urgency shows the severity's symbol and label in ink on a tint of its status color (INV-UI-011, FEED-REC-004). |
| Inventory badge | `frontend/src/components/InventoryBadge.tsx` | One chip per recommendation: ✓ On hand · △ On hand, running low · ✕ Need to buy · ◐ Partly on hand; tooltip lists each need's product or "not on shelf". Used by the dashboard `AlertCard` and the Recommendations list and detail pane. |

Colors use the status palette (`#0ca30c` / `#fab219` / `#d03b3b`) and are never shown without symbol and label. `frontend/src/types/inventory.ts` mirrors the Rust types and holds the label, color and symbol maps.

## Failure and Degradation

- **OpenRouter absent or failing**: products still save by hand (as `Other` when the owner gave no category and the assistant failed); refresh returns 503; nothing else on the shelf depends on the LLM.
- **Products query failing**: the feed is returned without inventory badges and the failure is logged (INV-ENRICH-005); dashboard alerts and Recommendations keep working. The shopping list, which reads the shelf itself, fails with a plain-language error.
- **No shelf / no needs**: recommendations carry no `inventory`; the shopping list lists Buy items only for recommendations whose needs are missing.

## Decisions & Alternatives

| Decision | Chosen | Alternatives Considered | Rationale |
|---|---|---|---|
| Stock model | Hand-set In stock / Low / Out | Amounts, units, a movements ledger, deduction per application | Answers "do I need to buy this" without bookkeeping the owner will not keep up; quantities can be added later as a column plus a movements table. |
| Saving without the LLM | Manual entry is first-class; profile optional and generable later | Refuse to save without OpenRouter (as landscape plants do) | Owner decision: the shelf must not depend on an external service. |
| Label rate | Owner-recorded (LLM may suggest one); stored, shown with "verify against your label", pre-fills the application form only | No label rate stored at all; label rates in advice | This is the HLD label-rate rule: advice never states a product's label rate, because the label is the legal authority on how much product to apply. A recorded rate saves retyping it on the form and never reaches advice. Reporting it back when the owner asks about the product is answering a question about their own data, not advice. |
| FRAC precedence | Curated `frac_classes_for_product` beats the LLM's FRAC list | Trust the LLM; union of both | FRAC classes drive protection and rotation; the curated table is verified, the LLM is not. |
| Facts vs profile | Facts copied once at add, then owner-owned; refresh returns a diff | Re-derive facts from each profile; overwrite on refresh | Owner corrections must survive regeneration. |
| Cross-category facts | `scoped_to_category` on add, refresh and PUT, plus a matching data migration | Store whatever the LLM returns | The LLM tags herbicides with FRAC classes and fungicides with nutrient targets; storing them as facts is noise that could match needs they cannot meet. |
| Matching strictness | A constrained axis requires the fact to be recorded | Treat an unknown fact as a match | A false "on hand" sends the owner for the wrong bag; an unknown is honestly "not on shelf". |
| Need shape | Multiple categories and any-of lists per axis, plus `optional` | One category per need (the plan's shape) | Soil-test and seeding needs are satisfiable by more than one category or species. [inferred] |
| Target vocabulary | Closed `ProductTarget` enum, enumerated in the LLM schema | Free-text targets | Free text makes matching impossible. |
| Shopping-list horizon | Active feed only | Seasonal-plan lookahead (next 45 days) | Owner decision: every upcoming use is already an active recommendation (timing windows enter as Info while opening soon), and plan activity categories are prose strings. |
| Restock vs Buy | A Low/Out product that fits suppresses the generic Buy line | List both | The restockable product is the concrete thing to buy. [inferred] |
| Where the inventory pass runs | In the feed after all sources and the nitrogen guard, before persisted answers | Per-page matching; after answers | One answer everywhere; enrichment must not change ids or severities that answers key on. |
| Shopping list as a view | A derived endpoint and panel | A new recommendation category | It summarizes existing advice rather than adding alerts. |
| Enum representation | `string_enum!`: variant name = serde = DB = schema value | Separate mapping tables per layer | One spelling across Rust, Postgres, the LLM schema and TypeScript removes a class of drift bugs. [inferred] |
| Application link | Nullable `product_id ON DELETE SET NULL` plus a copied `product_name` | Name-only linkage; hard FK requiring a product | History survives renames and deletes; the duplicate key stays unchanged. |
| OpenRouter attribution | `HTTP-Referer` + `X-Title` on every request | No headers | Owner rule: per-app traffic must be attributable on the OpenRouter console. |
| Disease shelf use | Inventory breaks ties only among equally efficacious eligible classes | Prefer owned classes outright | Owning a bag must never override efficacy or FRAC rotation. |
| Need alternatives | A need may be a set of alternatives, any one of which satisfies it | Loosen scoping so one product carries both a target and an amendment kind; split the declaration into two required needs | Scoping keeps stored facts honest; two required needs would demand both a supplement and lime when either will do. |
| Edit-form targets | Offer exactly the target family the backend keeps for the category | A broader or narrower hand-picked list per category | A checkbox the save silently drops, or a target the form strips, makes the owner's edit lie. |
| Implausible LLM values on add | Drop the one out-of-range value and save the product | Reject the add; save the value and let the CHECK fail | The owner asked to add a product; one bad suggestion should cost that value, not the product. |
| Assistant failure with no category | Save as `Other` with the `profile_error` notice | Return 503 and save nothing | The owner's entry is never lost to an external outage; no need lists `Other`, so the product claims nothing until the owner sets its category in Edit. |
| Running-low judgment | Only products serving required needs decide Low vs On hand | Every match, optional ones included | Optional needs never make a recommendation "need to buy", so they should not mask or create "running low" either. |
| Duplicate names | 409 on add or rename to an existing name (case-insensitive, archived included) | Allow duplicates | Link past applications, log suggestions and the shopping list all key on the name; two same-named products make each ambiguous. |
| Shelf read failure in the feed | Feed without inventory badges, warning logged | Fail the whole feed | HLD graceful degradation: the shelf annotates advice, it does not gate it. |

## Open Questions & Future Decisions

### Resolved

1. ✅ Quantities are out of scope (owner).
2. ✅ Products save without the LLM (owner).
3. ✅ The LLM may suggest a label rate; it is held to the product label-rate rule above (owner).
4. ✅ Insect control is its own category (owner).
5. ✅ The shopping list reads only the active feed (owner).
6. ✅ LLM facts are scoped to the product's category (owner).

### Deferred

1. **Need alternatives do not exist** (INV-NEED-006, deferred). Intent: a need may be a set of alternatives, so a magnesium need is met by a Supplement with a Magnesium target or by dolomitic lime. `ProductNeed` is one all-of need: axes are all-of (`backend/src/models/inventory.rs:85-109`), and scoping (`backend/src/models/product/profile.rs:117`) clears a Supplement's amendment kind and every target on a SoilAmendment (`enums.rs:159`), so such a need written as one `ProductNeed` could be met only by an `Other` product carrying both. No feed source declares one, because soil-test advice is not in the feed. Alternatives must exist before soil-test advice rejoins the feed.
2. **Divergence: edit-form target families differ from `fits_category`** (INV-UI-007). Intent: the edit form offers exactly the targets the backend keeps for the category. `TARGETS_BY_CATEGORY` (`frontend/src/types/inventoryTargets.ts:51-52`) offers Calcium/Magnesium on SoilAmendment, which the backend drops on save (`enums.rs:159`), and omits Boron/Copper/Calcium on Fertilizer, which the edit form strips on save if present (`EditProductForm.tsx:109`). Separately, the edit form clears N-P-K outside Fertilizer/Supplement (`EditProductForm.tsx:95-106`), which the backend does not scope (INV-SHELF-007); which side should change is open.
3. **Divergence: CHECK violations surface as 500** (INV-API-011, INV-LLM-010). Intent: PUT rejects an out-of-range rate or N-P-K with a plain 400; add drops an out-of-range LLM suggestion and saves the product. PUT validates only that rate amount and unit are paired (`backend/src/api/products.rs:189`); add validates nothing from the LLM. A rate ≤ 0 or N-P-K outside 0–100 fails the table's CHECK and maps to a generic 500 (`backend/src/error.rs:61`); on add this loses the product.
4. **Divergence: add with the assistant and no category fails on LLM error** (INV-LLM-011). Intent: the product saves as `Other` with the `profile_error` notice. `create_product` returns 503 and saves nothing (`products.rs:114-118`).
5. **Divergence: Upstream detail in responses.** (HLD goal: human-readable errors; upstream detail belongs in the log). `profile_error` and refresh 503s carry the OpenRouter error text, including the provider's response body (`backend/src/datasources/openrouter.rs:134`) and the raw model output (`openrouter.rs:155`); the UI shows it in a notice.
6. **Divergence: `Low` ignores which need a match serves** (INV-ENRICH-002). Intent: only products serving required needs decide Low vs On hand. "Every match Low" (`models/inventory.rs:198`) counts optional-need matches, so an in-stock optional product makes a recommendation whose required needs are covered only by Low products read OnHand.
7. **Divergence: duplicate product names are allowed** (INV-API-013). Intent: adding or renaming to a name already on the lawn's shelf (case-insensitive, trimmed, archived included) is a 409. There is no uniqueness check or constraint, so Link past applications assigns the rows to whichever same-named product links first (`backend/src/db/product_queries/link.rs:54-56`), and the shopping panel keys rows by `kind-label` (`ShoppingListPanel.tsx:71`), so two same-named Restock products collide as React keys.
8. **Divergence: missing rows are not reported** (INV-API-014). Intent: DELETE of an unknown id is a 404. DELETE returns 204 for an unknown id (`backend/src/db/product_queries.rs:159`). Separately, `update_product_profile` (`product_queries.rs:138`) does not check rows affected, so a refresh racing a delete reports success.
9. **Usability predicate is duplicated.** Disease risk re-implements "Fungicide, not archived, not Out" (`backend/src/api/disease_risk.rs:86-91`) instead of using `ProductNeed::matches`.
10. **Hand-kept mirrors.** `ALL_FRAC` (`backend/src/datasources/openrouter/product.rs:151`) lists FRAC classes by hand, so a new `FracClass` variant is not offered to the LLM until added. `categoriesFor` (`frontend/src/types/inventoryCompat.ts:10`) mirrors `accepts` by hand with no cross-language test.
11. **Divergence: a products-table failure takes down the whole feed** (INV-ENRICH-005). Intent: the feed is returned without inventory badges and a warning is logged. The feed reads the shelf with `?` (`recommendation_feed.rs:70-71`), so dashboard alerts, Recommendations and the shopping list all fail.
12. **Attribution values are hardcoded constants** (`openrouter.rs:9-10`) rather than read from configuration or `Cargo.toml`.
13. **Concurrent edits are last-write-wins.** The stock pill and Archive send the full product from the page's copy (`frontend/src/components/inventory/useProductActions.ts:56`).
14. **Size limits.** `frontend/src/types/inventory.ts` is 344 lines (over 300). Functions over 50 lines: `create_product` (~95), `shopping::build` (~62), `product_json_schema` (~58), `ProductRow::into_product` (~56), and the `Inventory`, `ProductRow`, `AddProductForm` and `EditProductForm` components.
15. **MCP reporting of a stored label rate.** The `products` tool returns the stored rate (`mcp/tools/shelf.rs:33`). The intended rule is that an agent states it only when the owner asks about that product, with "verify against your label", and never inside advice. The doctrine wording that enforces this is owned by the MCP server (its Deferred 11).
16. **Divergence: shopping-list urgency shows its label only in a tooltip** (INV-UI-011, FEED-REC-004). Intent: the severity's symbol and label in ink on a tint of its status color. `ShoppingListPanel.tsx:83-87` renders the symbol in the severity color with the severity name as a `title`.

## References

- `docs/plans/product-inventory.md`: agreed plan and owner decisions.
- `docs/agronomy-methods.md` § "Recommendations and the shelf" (served as an MCP resource; keep in sync with matching rules).
- HLD: `docs/high-level-design.md` (Non-Goals: label rates, quantity tracking; Key Design Decisions: no stock quantities; LLM supplies product facts with curated data taking precedence).
- Boundary components: Recommendation feed and state (FEED), Applications (APP), Disease risk (DIS), Timing windows (TIME), Agronomic rules (RULE), Soil tests (SOIL), Landscape plants (PLANT), MCP server (MCP), Web UI (UI).
