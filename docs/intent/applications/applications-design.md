---
parent: high-level-design
prefix: APP
---

# Lawn Profile & Application Log

## Context and Design Philosophy

This component is the owner's record of the lawn: one **lawn profile** (what the lawn is) and the **application log** (everything done to it and to the landscape plants). Under the HLD tenet *infer from the log, not from settings*, the log is the lawn's memory. Protection windows, FRAC rotation, the nitrogen budget, the rule that seeding and pre-emergent never share a season, plant-task completion and follow-up reminders are all read from it. The owner does not keep separate state in sync.

Guiding principles:

- **Record what happened, faithfully and cheaply.** Every field except type and date is optional. The form pre-fills what it can (shelf product, FRAC lookup, today's local date) and never blocks a save on something the owner does not know.
- **History is immutable to outside changes.** A row copies the product name at log time, so renaming or deleting a shelf product, or deleting a plant, does not rewrite what was applied.
- **Reject only true duplicates.** A row is a duplicate when it has the same profile, type, date, product name and plant. Tank mixes and two products of one type on one day are legitimate.
- **This component stores the facts; consumers interpret them.** The log stores FRAC classes, N-P-K, rate and plant target. Disease risk decides what protects. Timing windows decide what blocks a season. Agronomic rules and the nitrogen budget decide what counts toward N. The one interpretation owned here is `turf_nitrogen_lbs`, so every consumer computes lawn nitrogen the same way.

The system is single-lawn. Every handler resolves "the" profile as the lowest-id row (`get_default_lawn_profile`, `backend/src/db/queries.rs`).

## Lawn Profile

### Model (`backend/src/models/lawn_profile.rs`)

| Field | Type | Notes |
|---|---|---|
| `name` | text | Free text. |
| `grass_type` | `GrassType` | `KentuckyBluegrass` · `TallFescue` · `PerennialRyegrass` · `FineFescue` · `Bermuda` · `Zoysia` · `StAugustine` · `Mixed` |
| `usda_zone` | text | Free text, not validated. Used only in LLM prompts for plant care plans and product profiles. |
| `soil_type` | `SoilType?` | `Clay` · `Loam` · `Sandy` · `SiltLoam` · `ClayLoam` · `SandyLoam`. Read by soil-test advice and timing companions. |
| `lawn_size_sqft` | f64? | Used to scale agronomic amounts (nitrogen, seed per 1,000 sq ft) to the lawn in feeding and seeding advice (the consumers fall back to a default). |
| `irrigation_type` | `IrrigationType?` | `InGround` · `Hose` · `None`. Stored and shown, but no logic reads it. |

`GrassType::is_cool_season()` is true for the four cool-season species **and `Mixed`**. A mixed lawn is a cool-season blend (fescue / bluegrass / rye), the usual seed mix. It therefore gets the cool-season rules, seeding windows and disease models. Warm-season types (`Bermuda`, `Zoysia`, `StAugustine`) are the only ones that return false. The enum accepts them, but warm-season lawns are unsupported (HLD Non-Goals): no behavior for them is designed. Consumers own what "cool-season" gates.

The enums' `FromStr` implementations are lenient (case-insensitive, spaced and short forms such as `tttf`, `kbg`, `prg`, `st. augustine`). The DB and JSON form is the serde variant name, constrained by `chk_grass_type` / `chk_soil_type` / `chk_irrigation_type` (`20240101000002_add_constraints.sql`).

### Creation from `LAWN_*` defaults

At startup `ensure_default_profile` (`backend/src/main.rs`) creates one row from `LawnConfig` (`backend/src/config.rs`) only if no profile exists. After that the env vars are ignored and Settings is the source of truth. If the table is ever empty while the server runs (a restored or truncated database), saving the Settings page creates the profile through `PUT /api/v1/profile`, so every page's "no lawn profile, set one up in Settings" guidance leads somewhere that works. The API never deletes the profile.

| Env var | Default | Bad value |
|---|---|---|
| `LAWN_NAME` | `Main Lawn` | n/a |
| `LAWN_GRASS_TYPE` | `TallFescue` | warns, uses `TallFescue` |
| `LAWN_USDA_ZONE` | `7a` | n/a (free text) |
| `LAWN_SOIL_TYPE` | unset | warns, left unset |
| `LAWN_SIZE_SQFT` | unset | warns, left unset |
| `LAWN_IRRIGATION_TYPE` | unset | warns, left unset |

Every warning names the variable and the value it could not parse, so a typo in the compose file is visible in the log rather than a silently missing field. A bad value never stops the profile from being created.

A grass type stored in the DB that cannot be parsed reads back as `TallFescue` with a warning. A bad soil or irrigation type reads back as `None` with a warning (`LawnProfileRow::into_lawn_profile`).

### Profile API (`backend/src/api/profile.rs`)

| Method | Path | Behavior |
|---|---|---|
| GET | `/api/v1/profile` | The profile; 404 `No lawn profile found` if the table is empty. |
| PUT | `/api/v1/profile` | Partial update. An absent field is left alone. For `soil_type` / `irrigation_type`, `""` clears the field. For `lawn_size_sqft`, a value ≤ 0 clears it. An unknown enum value returns 400. The response re-reads the row so `updated_at` comes from the DB. When no profile exists, the same request creates it, with any omitted field filled as startup seeding does (`Main Lawn`, `TallFescue`, `7a`, optional fields unset). |

## Application Log

### Types and scope (`backend/src/models/application.rs`)

`ApplicationType::scope()` says where a type may be logged. The API enforces it. The frontend mirrors it in `frontend/src/types/index.ts` (`PLANT_REQUIRED_` / `UNIVERSAL_` / `TURF_ONLY_APPLICATION_TYPES`).

| Scope | Types | `plant_id` |
|---|---|---|
| TurfOnly | PreEmergent, PostEmergent, GrubControl, Overseed, Aeration, Dethatching, Lime, Sulfur, **Mowing** | must be null (400) |
| Universal | Fertilizer, Fungicide, Insecticide, Wetting, Other | optional (null = turf) |
| PlantRequired | Pruning, PlantFertilizer, Mulching, Deadheading, WinterProtection | required (400) |

**Mowing** is an ordinary turf-only type with no cut-height field. It appears in the list and the calendar like any other entry. No rule reads mowing entries.

`Application::is_turf()` is true when `plant_id` is null **and** the type is not PlantRequired. Lawn models (disease history, rotation, fall feeding, nitrogen) filter on it, so a plant application never protects or feeds the lawn.

### Nitrogen accounting

`Application::turf_nitrogen_lbs()` is the single definition of the lawn nitrogen an entry applied, in lb N per 1000 sq ft:

> `nitrogen_pct / 100 × rate_per_1000sqft`, only when the entry `is_turf()` and both values are present and > 0; otherwise `None`.

Consumers are the nitrogen budget endpoint, the nitrogen guard and soil-test advice. They sum the values and own the budget logic. Entries without N% or rate contribute nothing. The log never infers N from a product name.

`Application::is_feeding()` is the single definition of a **feeding**: an entry that `is_turf()`, of type `Fertilizer`, with `nitrogen_pct` unrecorded or above 0. Plant fertilizer and potash-only (0-0-x) products are not feedings; an entry with no analysis recorded is assumed to carry nitrogen. Fall fertilization's spacing and count, spring nitrogen's "fed this spring" and red thread's "nitrogen-hungry" all read it; each consumer applies its own date range. Today each consumer filters for itself, and spring nitrogen and red thread count any `Fertilizer` row (Deferred 20).

### Row shape

| Field | Notes |
|---|---|
| `application_type`, `application_date` | Required. The date is a calendar date (`YYYY-MM-DD`) with no time. |
| `product_name` | Free text, or copied from the shelf product (see *Shelf link*). Part of the duplicate key. |
| `product_id` | Optional shelf link; FK `ON DELETE SET NULL`. |
| `rate_per_1000sqft`, `coverage_sqft`, `nitrogen_pct`, `phosphorus_pct`, `potassium_pct` | Optional numbers. Rate and coverage may not be negative; an N, P or K percentage may not exceed 100 (400 with a plain message naming the field). |
| `frac_classes` | Optional `TEXT[]`; fungicides only (see *FRAC classes*). |
| `plant_id` | Optional plant target; FK `ON DELETE SET NULL`. |
| `follow_up_date` | Optional; must be ≥ `application_date`. |
| `notes` | Free text. |
| `created_at` | Set on create, preserved on update. |

JSON omits `plant_id`, `follow_up_date`, `frac_classes` and `product_id` when null (`skip_serializing_if`).

An application date may be in the future (a planned entry); consumers that judge what has happened already bound their reads by today (disease history uses `≤ today`).

The row carries no weather. The weather lake holds the station's conditions for every day, so anything that wants the conditions on an application date reads the lake by that date; a per-row copy would only duplicate it, and an entry logged after the fact would capture the wrong day's weather.

## Persistence

Migrations live in `backend/src/db/migrations/`. The ones this component owns:

| Migration | Effect |
|---|---|
| `20240101000000_initial_schema.sql` | `lawn_profiles`, `applications` (core columns plus the weather snapshot columns). |
| `20240101000001_indexes.sql` | `idx_applications_lawn_profile_id`, `idx_applications_date`. |
| `20240101000002_add_constraints.sql` | Enum CHECKs on the profile and `chk_application_type`; `idx_applications_profile_date`. |
| `20240101000005_add_npk_and_gdd.sql` | `nitrogen_pct`, `phosphorus_pct`, `potassium_pct`. |
| `20260425000000_plants.sql` | `applications.plant_id` FK + index; widens `chk_application_type` to include Mowing and the plant types. |
| `20260426000000_application_followup.sql` | `follow_up_date` + partial index. |
| `20260921000000_application_unique_by_product.sql` | The current duplicate guards (below). |
| `20260921000200_application_frac_classes.sql` | `frac_classes TEXT[]`. |
| `20260922000100_application_product.sql` | `product_id` FK (`ON DELETE SET NULL`) + index. Depends on INV's `products` table. |

Every enum is stored as its serde variant name (`enum_to_db_string`). A new `ApplicationType` therefore needs a migration that widens `chk_application_type`. A stored type the code cannot parse reads back as `Other` with a warning.

### Duplicate key

Two unique indexes, `applications_unique_turf` (rows with no plant) and `applications_unique_plant` (rows with a plant, which adds `plant_id` to the key), cover profile, type, date and the **normalised product name**:

```sql
lower(regexp_replace(btrim(COALESCE(product_name, '')), '\s+', ' ', 'g'))
```

A null and an empty product name collide. Case, leading and trailing whitespace and runs of inner whitespace do not distinguish names, so `"Heritage G"` and `"heritage  g "` are the same entry; this matches how product inventory matches log names to shelf products. `product_id` is not part of the key. A violation on create **or** update surfaces as **409** with `{"error": "An identical entry (same type, product and date) is already logged. Edit the existing entry, or change the product name if this was a different product."}` (`backend/src/error.rs`).

That message is reserved for these two indexes: the error mapping checks the violated constraint's name. A unique violation on any other constraint is not an application duplicate; its owning handler reports it, or it is an internal error.

Deleting a plant never fails on these indexes. Its entries lose their plant link (`ON DELETE SET NULL`) and stay in the log as history, so their placement under the key must not let them collide with each other or with lawn entries; the plant delete has no way to resolve such a conflict and the owner has nothing to correct.

### Query functions (`backend/src/db/queries.rs`)

| Function | Order / filter | Used by |
|---|---|---|
| `get_applications_page` | type filter in SQL **before** LIMIT/OFFSET; `application_date DESC, id DESC` | `GET /applications`, MCP `applications` tool |
| `get_applications_for_profile` | `application_date DESC`, limit/offset | recommendation feed (newest 1000), dashboard (newest 5) |
| `get_applications_for_profile_in_range` | `[start, end)` on `application_date` | nitrogen budget, disease risk, timing, seasonal plan, soil tests, MCP |
| `get_applications_or_follow_ups_in_range` | applied **or** follow-up due in range | calendar |
| `get_application_by_id`, `create_application`, `update_application`, `delete_application` | n/a | CRUD |
| `get_default_lawn_profile`, `create_lawn_profile`, `update_lawn_profile` | lowest id | profile |

Unknown FRAC codes stored in `frac_classes` are dropped with a warning on read rather than failing the row (`parse_frac_classes`).

## API Endpoints (`backend/src/api/applications.rs`)

| Method | Path | Request | Success | Errors |
|---|---|---|---|---|
| GET | `/api/v1/applications` | `?type=&scope=&limit=&offset=` | 200 `Application[]` | 400 unknown type or scope; 404 no profile |
| POST | `/api/v1/applications` | `CreateApplicationRequest` | 201 `Application` (with id) | 400 validation / out-of-range number / category mismatch / product no longer on the shelf; 404 no profile; 409 duplicate |
| PUT | `/api/v1/applications/{id}` | `CreateApplicationRequest` (full replace) | 200 `Application` | 404 unknown application; 400; 409 |
| DELETE | `/api/v1/applications/{id}` | n/a | 204 | 404 unknown application |

The list's `limit` defaults to 50 and is clamped to 1–200. A negative `offset` becomes 0. `type` is parsed with the lenient `FromStr` (`pre-emergent`, `grub control`, `mow` all work). `scope` is `turf` (no plant) or `landscape` (has a plant); absent means both. Both filters are SQL `WHERE` clauses applied before LIMIT/OFFSET, so a page is always full of matching rows.

**Create and update validation**, in order: parse type → parse date (`%Y-%m-%d`) → scope check against `plant_id` → numeric ranges (rate and coverage ≥ 0; N, P, K ≤ 100) → parse and check `follow_up_date ≥ application_date` → resolve the shelf link → write. Every error body is `{"error": "<message>"}`. The scope check has one exception on update: an entry whose plant was deleted (a plant-required type stored with no plant) may be saved without a plant as long as its type is unchanged, so a deleted plant's history stays editable.

**Update is a full replace** of every user field. An omitted optional field is cleared; only `lawn_profile_id` and `created_at` come from the existing row. When the update changes or clears `follow_up_date`, the handler deletes the stored answer for `application_followup_<id>`: the reminder for a new date is a new commitment and starts unanswered.

**Delete** of an id that does not exist returns 404, like update, so a stale page learns the entry is already gone.

`/api/v1/applications/calendar` is mounted under this path but belongs to the seasonal plan and calendar component. It reads the log through `get_applications_or_follow_ups_in_range`.

## Shelf Link (`backend/src/api/applications/product_link.rs`)

When `product_id` is present, `resolve`:

1. Loads the product and requires it to belong to this lawn. A missing product, or one from another lawn, returns **400** `That product is no longer on your shelf`. The request names a product the owner picked, typically from a form opened before the product was deleted, so this is a validation error the owner can fix by picking again, not a missing resource.
2. Requires `product.facts.category.accepts(application_type)`. Otherwise it returns **400** `"<name> is a <category> product and cannot be logged as <type>"`.
3. Returns the product id and a name. A non-blank `product_name` in the request wins; otherwise the product's name is copied.

`ProductCategory::accepts` is owned by product inventory (`backend/src/models/product/enums.rs`). The frontend mirrors it in `categoriesFor` (`frontend/src/types/inventoryCompat.ts`). `Other` on either side accepts anything:

| Application type | Accepted categories (+ `Other`) |
|---|---|
| Fertilizer, PlantFertilizer | Fertilizer, Supplement |
| Fungicide | Fungicide |
| PreEmergent, PostEmergent | Herbicide |
| GrubControl, Insecticide | InsectControl |
| Overseed | Seed |
| Lime, Sulfur | SoilAmendment |
| Wetting | Surfactant |
| Other | every category |
| Mowing, Aeration, Dethatching, plant types except PlantFertilizer | `Other` only |

The name copy makes history survive a rename or delete. On delete, `product_id` becomes null and the row keeps its name. Product inventory reads `product_id`. Its `POST /products/:id/link-applications` and `/inventory/suggestions` match on `product_name` (case-insensitive, trimmed). Those endpoints are inventory's.

## FRAC Classes (form field and storage)

`frac_classes` records the classes the owner read off a fungicide label (`FracClass` serde names such as `Frac11`, `FracM5`). The handler keeps them **only** for `Fungicide` entries and stores an empty pick as `NULL` (`fungicide_classes`). `NULL` means *not recorded*, and disease risk then falls back to resolving the product name. A premix records several classes. How a recorded class protects, and how it counts toward rotation, belongs to disease risk.

The form gets its options and the name lookup from `GET /api/v1/frac-classes?product=` (`backend/src/api/frac_classes.rs`): 12 classes with labels and example products, plus `matched` from the curated `frac_classes_for_product` resolver.

## Follow-up Reminders (`backend/src/logic/follow_up.rs`)

Any application may set `follow_up_date` ("do this again on…"). `generate_follow_up_recommendations` is a pure function over the log, the plants and `today`. The recommendation feed runs it, passing the newest 1000 applications and the server's local date (`Local::now()`, governed by `TZ`; `api/recommendation_feed.rs:54`), the one "today" the HLD gives every component.

| Condition | Result |
|---|---|
| No `follow_up_date` or no id | nothing |
| `today < follow_up − 7 d` (`LEAD_DAYS`) | nothing yet |
| `today > follow_up + 90 d` (`STALE_DAYS`) | dropped as stale |
| An application exists with the same type and the same `plant_id` (any product), dated after the original and no earlier than `follow_up − 7 d` | completed, so nothing |
| Otherwise, before the date | **Advisory** "scheduled in N day(s)" |
| Otherwise, on or after the date | **Warning** "scheduled for today" / "overdue by N day(s)" |

The recommendation id is `application_followup_<application id>`, in category `ApplicationTiming`. The title is `Follow-up: <type> on <plant common name | Lawn> (<product | application>)`. Data points are Target, Original, Follow-up and Product (when named), all with source `Manual`. The action asks the owner to re-apply and log it so the reminder clears. Follow-ups declare no product need. The recommendation feed applies answered state; its policy gives `application_followup_*` no expiry, only escalation (Advisory → Warning re-shows an answer given at Advisory). Because that answer never expires, the applications update handler deletes it whenever the follow-up date changes or is cleared, so moving a dismissed follow-up later brings it back on the new date.

Completion counts only entries in the follow-up's own window, from the 7-day lead-in onward. An entry logged the day after the original (a touch-up, a second product) is not the planned re-application and does not silence it. The calendar's follow-up markers belong to the seasonal plan and calendar component.

## UI

### Applications page (`frontend/src/pages/Applications.tsx`, route `/applications`)

- **List:** a table with Date, Type badge, Target (plant name or a "Turf" tag), Product, Rate/1k sqft, N-P-K, Coverage, Follow-up, Notes, and Edit / Delete. Pages of 50 come from the server with "Load older entries". The **Type** filter (`?type=`) and the **Scope** toggle (All / Turf / Landscape, `?scope=`) are both server-side; changing either reloads from the first page, so "Landscape" never shows an empty list while older pages hold plant entries. Delete asks for confirmation.
- **Form** (`ApplicationForm`, keyed on the edited id so it remounts):
  - The date defaults to `todayLocalISO()`.
  - The plant select appears for non-turf-only types and is required for plant-required ones, except when editing an entry whose plant was deleted with its type unchanged. Switching to a turf-only type clears it.
  - `ProductPicker`, then `FracClassPicker` for Fungicide. Changing the type drops a linked product whose category does not accept the new type (`categoriesFor`) and keeps its name as free text, so the form never submits a pair the API rejects.
  - Rate, coverage, notes, N/P/K %.
  - "Schedule a follow-up", with `min` set to the application date and +2 / +4 / +6 week shortcuts computed with `addDaysISO`.
  - Client checks mirror the API: plant required; follow-up set and ≥ date. Empty fields are sent as absent. `frac_classes` is sent only for Fungicide and only when non-empty.
  - **Out → In stock:** after a successful save, if the product **picked in this form session** has `stock_status === "Out"`, the form asks *"<name> is marked Out in your inventory. Mark it In stock?"* and on yes calls `markProductInStock`. This is the only stock automation. The save stands whatever the stock call does: the form closes and the list refreshes, and a failed stock update is reported as its own message, so the owner never resubmits an entry that is already logged.
  - Errors show above the list as `"<status>: <body>"` (`fetchJson` in `frontend/src/api/client.ts`).

### ProductPicker (`frontend/src/components/ProductPicker.tsx`)

The picker is the form side of the shelf link. It loads every product, archived included. It offers products whose category is in `categoriesFor(type)` and that are not archived, plus the currently linked product even when it is archived or no longer compatible. Options are labeled with brand, "Other" and stock state. The choices are "— none —" (clears the name), a product, or "Not in inventory — type a name…", which reveals a free-text name input. With an empty shelf the picker is just the free-text field. Picking a product pre-fills the name, N-P-K, FRAC classes and the label rate when the product has one. Every pre-filled field stays editable.

### FracClassPicker (`frontend/src/components/FracClassPicker.tsx`)

The picker shows a checkbox grid of the 12 classes, with example products as tooltips. After a 300 ms debounce on the product name it calls `/frac-classes?product=` and pre-fills `matched`, unless the value is already `recorded` (editing an entry with classes, or a shelf product picked) or the owner has touched a box. The form remounts it (`key={productId ?? "manual"}`) on a pick, so a product's confirmed classes are not overwritten by the lookup. When the name is non-empty, unmatched and nothing is ticked, it warns that without a class the entry earns no disease protection or rotation credit. A failed lookup leaves the picker usable by hand.

### Settings page (`frontend/src/pages/Settings.tsx`, route `/settings`)

The page edits the profile: name and USDA zone are required, plus grass type, soil type ("Not specified" sends `""`), size (empty sends `0`, which clears it) and irrigation type. It shows a 3-second success banner and the created / updated timestamps. When no profile exists, the page shows the empty form with a note that saving creates the lawn profile, rather than a load error; Save Profile then creates it through the same `PUT`.

### Local dates (`frontend/src/utils/dates.ts`)

`todayLocalISO()` and `addDaysISO()` work on the **browser's local calendar**. The system's "today" is the server's local date; the two agree whenever the browser and the server share a timezone, which is the normal single-owner setup. `toISOString()` would report the UTC date, already tomorrow from 8 pm Eastern, so an evening entry would be stored a day late and ignored as future-dated by the disease and timing models.

## Failure and Degradation

- **No profile row:** the application endpoints and `GET /profile` return 404, and pages point the owner to Settings, where saving creates the profile (`PUT /profile`). Startup normally prevents this.
- **Shelf unreachable in the form:** `ProductPicker` treats a failed product list as an empty shelf and shows the free-text field. Logging still works.
- **FRAC lookup fails:** the picker works by hand. The backend falls back to the name when no classes are recorded.
- **Plant deleted:** the delete always succeeds. `plant_id` becomes null and the row keeps its type, so PlantRequired entries are still not `is_turf()` and stay out of lawn models, and they remain editable without a plant.
- **Product deleted:** `product_id` becomes null and `product_name` remains.
- **Unparseable stored values:** type becomes `Other`, unknown FRAC codes are dropped, an unknown grass type reads as `TallFescue`. Each logs a warning, and the read never fails.

## Decisions & Alternatives

| Decision | Chosen | Alternatives Considered | Rationale |
|---|---|---|---|
| Duplicate key | `(profile, type, date, normalised product name)`, split turf / plant (plant adds `plant_id`); violation → 409 with guidance | `(profile, type, date)` unique; no uniqueness | Two products of one type on one day (tank mix, starter + second product) is a legitimate log; only a true double-submit should collide, and it should say so rather than 500. |
| Product name comparison in the key | Case-insensitive, trimmed, inner whitespace collapsed | Exact match; include `product_id` | A double-submit typed slightly differently is still a double-submit, and inventory already matches log names this way; two components disagreeing on "same product" would confuse the owner. |
| Duplicate-message scope | Only `applications_unique_turf` / `applications_unique_plant` map to the duplicate message | Map every unique violation to it | Another table's unique index (a plant delete, a future constraint) would otherwise report a misleading "identical entry is already logged". |
| Dates | Follow-up "today" is the server's local date (`TZ` = the lawn's timezone); the form defaults to the browser-local date | UTC date; station-local date from the lake's offset; the browser date as the system clock | Every agronomic date is a lawn-local calendar day, and one configured clock gives every component the same day. A UTC date is a day ahead in the evening and makes fresh entries look future-dated. The browser default is a convenience that matches the server date when both share a timezone. |
| Shelf link shape | Nullable `product_id` FK `ON DELETE SET NULL` plus `product_name` copied at log time | `product_id` only; name only | History must survive a product rename or delete, and the duplicate key stays on the name. Owner decision (product inventory plan). |
| Product/type compatibility | `ProductCategory::accepts` gates both the picker and the API (400) | Allow any product under any type | Prevents logging a herbicide as a fungicide, which would corrupt protection and rotation. Owner decision (product inventory plan). |
| Stock automation on logging | Offer "mark In stock?" only when the picked product is `Out` | Auto-decrement or auto-set; no prompt | No quantity tracking by owner decision. Logging use of an Out product implies it was rebought. |
| FRAC classes on the entry | Optional `TEXT[]`, fungicides only, empty means `NULL` = not recorded, name resolver as fallback | Name resolver only; required field | An unrecognised product silently earned no protection. Recording from the label fixes that without forcing it, and premixes need several classes. |
| Landscape vs lawn | An application logged against a plant is never a lawn entry, whatever its type, and keeps that status (with the plant's name) after the plant is deleted | Lawn = no `plant_id` and a non-plant-required type | Landscape work must never feed the lawn's nitrogen budget, feeding history, disease protection or plan completion; a deleted plant must not turn its history into lawn work. |
| One feeding definition | `is_feeding()` on the model (turf, Fertilizer, N% unrecorded or > 0) | Each consumer filters the log itself | The rules and disease risk must agree on whether the lawn has been fed; a potash-only or plant entry must not count for one and not the other. |
| One nitrogen definition | `turf_nitrogen_lbs()` on the model (turf only, N% × rate) | Each consumer computes N | Budget, guard and soil-test advice must agree. Plant fertilizer must never count toward lawn N. [inferred] |
| Plant applications in the same log | One `applications` table with `plant_id` and a scope per type | Separate plant-care log | One history, one calendar, one follow-up mechanism; `is_turf()` keeps lawn models clean. [inferred] |
| Mowing | An ordinary turf-only `ApplicationType`, no cut-height field | Separate mowing log with height | Mowing is recorded as history on the list and calendar; no logic needs height. [inferred] |
| Type and scope filter placement | Both SQL `WHERE` before LIMIT/OFFSET | Filter the fetched page in memory | A rare type, or the landscape entries, would hide behind a season of lawn entries and show an empty page. |
| Follow-up completion | Cleared by an application of the same type and plant scope dated after the original and from 7 days before the follow-up date | Any later entry; explicit "done" toggle; same product required | Infers completion from the log rather than a toggle, but only an entry in the follow-up's own window is the planned re-application. |
| Rescheduled follow-up | Changing or clearing the follow-up date deletes the stored answer | Keep the answer (it never expires) | A dismissal answered the old date; the new date is a new commitment that should surface. |
| Weather on an entry | None stored; read the lake by date | Snapshot columns filled at log time | The lake already holds every day's conditions; a snapshot taken when the entry is typed records the wrong day for a back-dated entry. |
| Numeric validation | 400 for negative rate or coverage and N/P/K above 100; future dates allowed | No checks; reject future dates | Impossible values would corrupt nitrogen accounting; a future date is a legitimate planned entry and consumers already bound by today. |
| Unknown `product_id` | 400 "That product is no longer on your shelf" | 404 `Product <id> not found` | The request is valid except for a stale pick; 404 reads as "the URL is wrong" and the message tells the owner what to do. |
| DELETE of an unknown id | 404 | 204 (idempotent) | Matches PUT and tells a stale page the entry is already gone. |
| Plant deletion and the log | The delete always succeeds; orphaned entries stay editable history | Block the delete; delete the entries | Removing a plant from the landscape must not depend on the log, and what was applied stays true. |
| "Mark In stock" failure | The save stands; the stock error is reported on its own | Treat the whole save as failed | The entry is already stored; a resubmit can only produce a duplicate 409. |
| Follow-up answered state | No expiry for `application_followup_<id>`; returns on escalation | Episode expiry like other recs | The id already names one event. Owner-decided escalation + expiry policy (recommendation feed). |
| Profile creation | Seeded once from `LAWN_*` at startup; `PUT /profile` creates the profile when none exists; no delete | GET/PUT update only; full profile CRUD; env always authoritative | Single lawn; Settings becomes the source of truth after first boot, and every page sends the owner there when no profile exists, so saving Settings must be able to create one. |
| `Mixed` grass | Cool-season | Neither cool nor warm | A mixed lawn is a cool-season blend, and treating it as neither silenced the rules, windows and disease models for the most common lawn. Owner decision. |

## Open Questions & Future Decisions

### Resolved
1. ✅ Two products of one type on one day are allowed; only an identical (type, date, product, plant) entry is a 409.
2. ✅ Date gates use the server's local date; default form dates are browser-local, which agrees with it when both share a timezone.
3. ✅ Applications link to shelf products by id, with the name copied. The only stock automation is the "Out → In stock?" offer.
4. ✅ `GrassType::Mixed` is cool-season; warm-season lawns are unsupported.
5. ✅ Advice never states a product label rate. A recorded label rate only pre-fills the form's rate field; follow-up reminders state no amounts. The rate an owner logs is a fact the nitrogen accounting reads, not advice.

### Deferred
1. **File and function size.** `backend/src/db/queries.rs` is 609 lines and mixes profile, application, recommendation-state and threshold-crossing queries; the latter two belong to other components. `frontend/src/pages/Applications.tsx` is 850 lines (`ApplicationForm` is ~370 of them) and `pages/Settings.tsx` is 304, all over the 300-line limit. `create_application` and `update_application` each run ~100 lines and duplicate the same validation. All three files should be split.
2. **Divergence: the duplicate key compares names exactly** (APP-DUP-001). `20260921000000_application_unique_by_product.sql` keys on `COALESCE(product_name, '')`, so `"Heritage G"` and `"heritage g "` are different entries. Intent: the key normalises case and leading, trailing and repeated whitespace.
3. **Divergence: the weather snapshot still exists** (APP-LOG-010, APP-API-005). The four snapshot columns, the `WeatherSnapshot` model and the request field remain, nothing fills them, and PUT keeps a stored snapshot when the request has none (`api/applications.rs:271`). Intent: a migration drops the columns and the field is removed from the model, request and response.
4. **Divergence: plant deletion can fail, and its entries cannot be edited** (APP-LOG-011, APP-LOG-012, APP-FORM-002, APP-FORM-004). `ON DELETE SET NULL` (`20260425000000_plants.sql:25`) moves plant rows into `applications_unique_turf`, so two same-day, same-type, same-name entries on two deleted plants make the plant delete fail with the application-duplicate 409. The scope check requires a `plant_id` for PlantRequired types (`api/applications.rs:217`), and the form blocks the save the same way. Intent: the delete always succeeds and the orphaned entries save without a plant while their type is unchanged.
5. **Divergence: every unique violation maps to the application-duplicate message** (APP-DUP-003). `error.rs:58` checks only `is_unique_violation()`. Intent: only the two application indexes produce that message.
6. **Divergence: no numeric validation** (APP-API-010). Negative rates or coverage and N/P/K above 100 are accepted (`api/applications.rs:105-148`). Intent: 400 with a plain message naming the field; future dates stay accepted.
7. **Divergence: DELETE returns 204 for an unknown id** (APP-API-011; `api/applications.rs:293-299`). Intent: 404, like PUT.
8. **Divergence: an unknown `product_id` returns 404 `Product <id> not found`** (APP-LINK-001; `api/applications/product_link.rs:20-23`). Intent: 400 `That product is no longer on your shelf`.
9. **Divergence: the Scope toggle filters client-side over server pages** (APP-LIST-003, APP-API-012). "Landscape" can show "No applications found" while older pages still hold plant entries (`Applications.tsx:110-115`), and `GET /applications` has no `scope` parameter. Intent: the server filters by scope before paging.
10. **Feed reads only the newest 1000 entries** (`RULE_HISTORY_LIMIT`, `api/recommendation_feed.rs:19`), ordered by date with no id tie-break (`db/queries.rs:104`). Follow-ups older than that are never evaluated, though the 90-day stale window makes this unlikely to matter.
11. **Divergence: rescheduling keeps a dismissed follow-up hidden** (APP-FOLLOW-008). The id is stable across edits and its answer never expires (`models/recommendation_state.rs:33`), and the update handler does not touch it. Intent: changing or clearing the follow-up date deletes the stored answer.
12. **`irrigation_type` is unused by any logic.** `usda_zone` reaches only LLM prompts. Keep both as descriptive data, or wire them in?
13. **Error display.** The form shows the raw `"409: {\"error\":…}"` string instead of the message.
14. **`/frac-classes` ownership.** It serves this component's form, but it exposes disease risk's curated resolver. Decide which LLD owns the endpoint.
15. **Divergence: changing the type keeps an incompatible product link** (APP-FORM-010). Nothing clears `productId` when the type changes, and the picker keeps the linked product in its options (`ProductPicker.tsx:57-60`), so saving returns the 400 category error (`product_link.rs:24`). Intent: the form drops the link and keeps the name as free text.
16. **Divergence: a failed "Mark In stock" reads as a failed save** (APP-FORM-011). The catch shows the error and skips `onSaved`, so the form stays open and resubmitting a create returns 409 (`Applications.tsx:444-457`). Intent: the save stands and the stock failure is reported on its own.
17. **Divergence: follow-up completion accepts any later entry** (APP-FOLLOW-003). `follow_up_completed` (`logic/follow_up.rs:16-23`) counts an entry dated the day after the original, long before the follow-up's lead-in. Intent: only entries dated from `follow_up − 7 d` onward complete it.
18. **Divergence: invalid optional `LAWN_*` values are dropped silently** (APP-PROF-006). A bad soil type, irrigation type (`main.rs:275-291`) or size (`config.rs:183-185`) gets no warning; only a bad grass type does. Intent: every invalid value logs a warning naming the variable.
19. **Browser and server dates can differ.** If the browser is in a different timezone from the server's `TZ`, the form's default date (and the +2/+4/+6 week shortcuts) can be a day off the server's "today" around midnight. Harmless in the single-owner setup; the form could default from a server-supplied date instead.
20. **No shared feeding definition** (divergence, APP spec pending). Fall fertilization filters `is_turf()` and N% (`fall_fertilization.rs:58-59`); spring nitrogen (`spring_nitrogen.rs:53-60`) and red thread's `days_since_fertilizer` (`logic/disease/history.rs`) count any `Fertilizer` row. Intent: all three read `Application::is_feeding()`.
21. **Divergence: the profile cannot be created through the API** (APP-PROF-011, APP-PROF-012, APP-SET-003). `update_profile` (`api/profile.rs`) returns 404 when no profile exists, and the Settings page shows that as a load error. Intent: `PUT /profile` creates the profile when none exists, and Settings presents the empty form as the way to create it.
22. **Divergence: a deleted plant's applications become lawn entries** (APP-LOG-002, APP-LOG-009). Deleting a plant sets `plant_id` to null (`ON DELETE SET NULL`), and `is_turf()` treats a null `plant_id` with a non-plant-required type as lawn, so a deleted shrub's Fertilizer or Fungicide entry starts counting toward lawn nitrogen, feedings and disease protection. Intent: a landscape application never counts toward the lawn, whatever its type; each entry keeps a record that it was for a plant (the plant's name).

## References

- `docs/high-level-design.md`: Applications component, "Infer from the log" tenet.
- `docs/plans/product-inventory.md`: "Linking to applications".
- `docs/agronomy-methods.md`: nitrogen per feeding (rate × N%, lawn only).
- Code: `backend/src/models/application.rs`, `backend/src/models/lawn_profile.rs`, `backend/src/api/applications.rs`, `backend/src/api/applications/product_link.rs`, `backend/src/api/profile.rs`, `backend/src/api/frac_classes.rs`, `backend/src/logic/follow_up.rs`, `backend/src/db/queries.rs`, `backend/src/error.rs`, `backend/src/main.rs` (`ensure_default_profile`), `backend/src/config.rs` (`LawnConfig`), `backend/src/db/migrations/`.
- Frontend: `frontend/src/pages/Applications.tsx`, `frontend/src/pages/Settings.tsx`, `frontend/src/components/ProductPicker.tsx`, `frontend/src/components/FracClassPicker.tsx`, `frontend/src/utils/dates.ts`, `frontend/src/types/inventoryCompat.ts`.
- Boundaries: product inventory (shelf, `ProductCategory::accepts`, link-applications), disease risk (FRAC protection and rotation), timing windows (seed / pre-emergent season blocking), recommendation feed (answered-state policy), seasonal plan and calendar (calendar endpoint and follow-up markers), landscape plants (plant targets).
