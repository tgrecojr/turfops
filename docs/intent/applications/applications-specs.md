# Lawn Profile & Application Log — EARS Specs

Design: [applications-design.md](applications-design.md). Prefix `APP`. Facets: `PROF` (lawn profile, startup seeding, `/profile`), `LOG` (application types, scope, row shape, the turf / nitrogen / feeding definitions), `DUP` (duplicate key), `API` (`/applications` endpoints and validation), `LINK` (shelf-product link), `FRAC` (FRAC classes on the entry and the form's picker), `FOLLOW` (follow-up reminders), `LIST` (Applications page list), `FORM` (application form and product picker), `SET` (Settings page).

"Turf entry" means `Application::is_turf()` is true: an application that was never logged against a landscape plant, whatever its type. "The profile" is the single lawn profile, the lowest-id row.

## Lawn profile

- [x] **APP-PROF-001**: The system shall keep one lawn profile, and every profile and application handler shall resolve it as the lowest-id `lawn_profiles` row.
- [x] **APP-PROF-002**: The lawn profile shall hold a name, a grass type, a USDA zone (free text, not validated), and optional soil type, lawn size in sq ft, and irrigation type.
- [x] **APP-PROF-003**: `GrassType::is_cool_season()` shall return true for Kentucky bluegrass, tall fescue, perennial ryegrass, fine fescue and `Mixed` (a cool-season blend).
- [x] **APP-PROF-004**: The system shall parse grass, soil and irrigation types case-insensitively, accepting spaced and short forms (such as `tttf`, `kbg`, `prg`), and shall store and serialize each as its serde variant name.
- [x] **APP-PROF-005**: When the server starts and no lawn profile exists, the system shall create one from the `LAWN_*` environment variables (defaults: name `Main Lawn`, grass type `TallFescue`, zone `7a`); when a profile exists, the system shall ignore the `LAWN_*` variables.
- [x] **APP-PROF-006**: If a `LAWN_*` environment variable cannot be parsed when seeding the profile at startup, then the system shall log a warning naming the variable and its value, use `TallFescue` for `LAWN_GRASS_TYPE`, leave the field unset for `LAWN_SOIL_TYPE`, `LAWN_IRRIGATION_TYPE` or `LAWN_SIZE_SQFT`, and still create the profile.
- [x] **APP-PROF-007**: If a stored profile's grass type cannot be parsed, then the system shall read it as `TallFescue` and log a warning; if its soil or irrigation type cannot be parsed, then the system shall read that field as unset and log a warning; the read shall not fail.
- [x] **APP-PROF-008**: When `GET /api/v1/profile` is called, the system shall return the profile, or 404 `No lawn profile found` when none exists.
- [x] **APP-PROF-009**: When `PUT /api/v1/profile` is called, the system shall update only the fields present in the request, clear `soil_type` or `irrigation_type` when sent as `""`, clear `lawn_size_sqft` when sent as a value ≤ 0, and respond with the profile re-read from the database (so `updated_at` is the stored time).
- [x] **APP-PROF-010**: If `PUT /api/v1/profile` carries a grass, soil or irrigation type the system cannot parse, then the system shall return 400 naming the unknown value and change nothing.
- [x] **APP-PROF-011**: The API shall offer no way to delete the lawn profile, and shall create it only through `PUT /api/v1/profile` when none exists (startup seeding from `LAWN_*` being the other way a profile comes to exist).
- [x] **APP-PROF-012**: When `PUT /api/v1/profile` is called and no lawn profile exists, the system shall create the profile from the request's fields, filling any omitted field as startup seeding does (name `Main Lawn`, grass type `TallFescue`, zone `7a`, the optional fields unset), apply the same parsing, clearing and 400 rules as an update, and respond with the stored profile.

## Application log model

- [x] **APP-LOG-001**: The system shall assign every application type a scope: turf-only (PreEmergent, PostEmergent, GrubControl, Overseed, Aeration, Dethatching, Lime, Sulfur, Mowing), universal (Fertilizer, Fungicide, Insecticide, Wetting, Other), or plant-required (Pruning, PlantFertilizer, Mulching, Deadheading, WinterProtection).
- [x] **APP-LOG-002**: `Application::is_turf()` shall return true only for an application that was never logged against a landscape plant, whatever its type, so no landscape application ever counts toward the lawn (nitrogen, feedings, protection, completion), including after its plant is deleted.
- [x] **APP-LOG-003**: `Application::turf_nitrogen_lbs()` shall return `nitrogen_pct / 100 × rate_per_1000sqft` (lb N per 1,000 sq ft) only for a turf entry whose N% and rate are both present and above 0, and otherwise none; nitrogen shall never be inferred from a product name.
- [x] **APP-LOG-004**: The system shall provide `Application::is_feeding()` as the single definition of a feeding: a turf entry of type Fertilizer whose `nitrogen_pct` is unrecorded or above 0, so plant fertilizer and potash-only (0-0-x) entries are not feedings and an entry with no analysis recorded is assumed to carry nitrogen.
- [x] **APP-LOG-005**: The system shall record Mowing as an ordinary turf-only application type with no cut-height field.
- [x] **APP-LOG-006**: An application shall require a type and a calendar date (`YYYY-MM-DD`, no time); every other field (product name, shelf product, rate, coverage, N-P-K percentages, FRAC classes, plant, follow-up date, notes) shall be optional.
- [x] **APP-LOG-007**: When serializing an application, the system shall omit `plant_id`, `follow_up_date`, `frac_classes` and `product_id` when they are null.
- [x] **APP-LOG-008**: If a stored application's type cannot be parsed, then the system shall read it as `Other` and log a warning; if a stored FRAC code cannot be parsed, then the system shall drop that code and log a warning; the read shall not fail.
- [x] **APP-LOG-009**: When a landscape plant is deleted, the system shall keep a record on each of its applications that it was for a plant (the plant's name), so the entry stays non-turf and its history still shows what it was for.
- [x] **APP-LOG-010**: An application shall store no weather snapshot: the `applications` table shall have no weather snapshot columns and the application request and response shall have no `weather_snapshot` field, because the weather lake holds the conditions for any application date.
- [x] **APP-LOG-011**: When a landscape plant is deleted, the delete shall succeed whatever applications were logged against the plant, and no application duplicate key shall reject it.
- [x] **APP-LOG-012**: When `PUT /api/v1/applications/:id` updates an entry whose plant was deleted (stored with a plant-required type and no plant), the system shall accept the update without a plant while the type stays the same, so a deleted plant's entries remain editable history.

## Duplicate key

- [x] **APP-DUP-001**: The system shall reject a second application with the same profile, type, date and product name among turf-targeted rows, and the same profile, type, date, plant and product name among plant-targeted rows, comparing product names without regard to case, leading or trailing whitespace, or runs of inner whitespace (a missing name equal to an empty one), and shall accept two different products of one type on one day.
- [x] **APP-DUP-002**: If creating or updating an application violates the duplicate key, then the applications endpoint shall return 409 with `An identical entry (same type, product and date) is already logged. Edit the existing entry, or change the product name if this was a different product.`
- [x] **APP-DUP-003**: The system shall return the application duplicate-entry message only for a violation of `applications_unique_turf` or `applications_unique_plant`; if any other unique constraint is violated, then the system shall not report it as an application duplicate.

## Applications endpoints

- [x] **APP-API-001**: When `GET /api/v1/applications` is called, the system shall return the profile's applications newest first (date descending, then id descending), `limit` defaulting to 50 and clamped to 1–200, a negative `offset` treated as 0, and an optional `type` filter applied before paging.
- [x] **APP-API-002**: If `GET /api/v1/applications` is called with a `type` the system cannot parse (parsing is case-insensitive and ignores spaces and hyphens), then the system shall return 400.
- [x] **APP-API-003**: When `POST /api/v1/applications` succeeds, the system shall return 201 with the stored application, including its id.
- [x] **APP-API-004**: If a `POST` or `PUT /api/v1/applications` request has an unknown type, a date or follow-up date not in `YYYY-MM-DD` form, a plant-required type without a plant, a turf-only type with a plant, or a follow-up date before the application date, then the system shall return 400 and store nothing.
- [x] **APP-API-005**: When `PUT /api/v1/applications/:id` is called, the system shall replace every user field from the request, clearing omitted optional fields, and keep only the row's profile, `created_at` and its deleted-plant marker (APP-LOG-009) from the stored row.
- [x] **APP-API-006**: If `PUT /api/v1/applications/:id` names an application that does not exist, then the system shall return 404.
- [x] **APP-API-007**: When `DELETE /api/v1/applications/:id` is called, the system shall delete the application and return 204.
- [x] **APP-API-008**: If no lawn profile exists, then `GET` and `POST /api/v1/applications` shall return 404.
- [x] **APP-API-009**: The applications and profile endpoints shall write every error message for the owner in plain language, naming fields as the form shows them (plant, follow-up date, date) rather than request field names (`plant_id`, `follow_up_date`), and shall report an internal inconsistency (such as a profile row without an id) as an internal error with the detail kept in the log.
- [x] **APP-API-010**: If a `POST` or `PUT /api/v1/applications` request has a negative rate per 1,000 sq ft or coverage, or an N, P or K percentage above 100, then the system shall return 400 with a plain-language message naming the field and store nothing; an application date in the future shall be accepted.
- [x] **APP-API-011**: If `DELETE /api/v1/applications/:id` names an application that does not exist, then the system shall return 404.
- [x] **APP-API-012**: When `GET /api/v1/applications` is called with `scope=turf` or `scope=landscape`, the system shall return only the entries without or with a plant respectively, filtering in SQL before paging together with any `type` filter; if `scope` has any other value, then the system shall return 400.

## Shelf-product link

- [x] **APP-LINK-001**: If an application create or update request carries a `product_id` that does not exist or belongs to another lawn, then the system shall return 400 `That product is no longer on your shelf` and store nothing.
- [x] **APP-LINK-002**: If an application create or update request carries a `product_id` whose category does not accept the application type (Product inventory's `ProductCategory::accepts`), then the system shall return 400 `<name> is a <category> product and cannot be logged as <type>`.
- [x] **APP-LINK-003**: When an application create or update request links a shelf product, the system shall store the request's product name when it is non-blank and otherwise copy the product's name onto the application.
- [x] **APP-LINK-004**: When a shelf product is deleted, the system shall set `product_id` to null on its applications and keep each row's copied product name, so a product rename or delete never rewrites the log.

## FRAC classes on the entry

- [x] **APP-FRAC-001**: When creating or updating an application, the system shall store `frac_classes` only for a Fungicide entry, as a list (several for a premix), and shall store an absent or empty list, or any classes sent with another type, as null (not recorded).
- [x] **APP-FRAC-002**: While the application form's type is Fungicide, the form shall show a FRAC class checkbox grid of the options from `GET /api/v1/frac-classes`, with each class's example products as its tooltip.
- [x] **APP-FRAC-003**: When the product name in the application form changes, the FRAC picker shall, after a 300 ms pause, look the name up and tick the matched classes, unless the entry already has recorded classes, a shelf product was picked, or the owner has ticked or unticked a box.
- [x] **APP-FRAC-004**: While the application form's product name is non-empty, the lookup matched no class and no box is ticked, the FRAC picker shall warn that TurfOps doesn't recognize the product and that without a class the application can't count toward disease protection or fungicide rotation.
- [x] **APP-FRAC-005**: If the FRAC lookup fails, then the application form shall still save the entry, recording only the classes ticked by hand (none when the grid never loaded), so the backend resolves classes from the product name.

## Follow-up reminders

- [x] **APP-FOLLOW-001**: The follow-up reminder source shall be a pure function of the application log, the landscape plants and today's date, emitting one recommendation per application that has a follow-up date and an id and is not yet completed or out of range.
- [x] **APP-FOLLOW-002**: The follow-up reminder source shall emit nothing for an application before 7 days ahead of its follow-up date, and nothing once today is more than 90 days past it.
- [x] **APP-FOLLOW-003**: The follow-up reminder source shall treat a follow-up as completed, and emit nothing for it, when the log holds an application of the same type with the same plant target (no plant = the lawn), whatever its product, dated after the original application and no earlier than 7 days before the follow-up date.
- [x] **APP-FOLLOW-004**: The follow-up reminder source shall emit Advisory before the follow-up date and Warning on or after it, with the description saying "scheduled in N day(s)", "scheduled for today" or "overdue by N day(s)".
- [x] **APP-FOLLOW-005**: Each follow-up reminder shall have the id `application_followup_<application id>`, category ApplicationTiming, and the title `Follow-up: <type> on <plant common name, or Lawn> (<product name, or application>)`.
- [x] **APP-FOLLOW-006**: Each follow-up reminder shall carry the data points Target, Original and Follow-up, plus Product when the application names one, all sourced Manual, and the action "Re-apply <type> to <target> and log it under Applications so this reminder clears."
- [x] **APP-FOLLOW-007**: A follow-up reminder shall declare no product need and state no amount or rate.
- [x] **APP-FOLLOW-008**: When `PUT /api/v1/applications/:id` changes or clears an application's follow-up date, the system shall delete the saved answer (dismissed / addressed) for `application_followup_<application id>`, so the reminder for the new date starts unanswered.

## Applications page list

- [x] **APP-LIST-001**: The Applications page shall list applications in a table with Date, Type badge, Target (the plant's name, or a "Turf" tag), Product, Rate/1k sqft, N-P-K, Coverage, Follow-up, Notes, and Edit and Delete buttons.
- [x] **APP-LIST-002**: The Applications page shall load applications from the server 50 at a time and, while the last page was full, offer "Load older entries" to fetch the next page.
- [x] **APP-LIST-003**: The Applications page's Type filter (`?type=`) and Scope toggle (All / Turf / Landscape, `?scope=`) shall both be applied by the server, and changing either shall reload the list from the first page.
- [x] **APP-LIST-004**: When the owner clicks Delete on the Applications page, the page shall ask for confirmation before deleting.
- [x] **APP-LIST-005**: If a request from the Applications page or the Settings page fails, then the page shall show a plain-language message that says what went wrong and what to do: the server's `error` message for a 400 or 409, a pointer to Settings when the Applications page finds no lawn profile, "try again" on a timeout; never a status code, a JSON body or internal detail.

## Application form

- [x] **APP-FORM-001**: The application form shall default a new entry's date to today on the browser's local calendar, never the UTC date.
- [x] **APP-FORM-002**: The application form shall show the plant select for every type that is not turf-only, require it for plant-required types except when editing an entry whose plant was deleted with its type unchanged, and clear the selected plant when the type changes to a turf-only type.
- [x] **APP-FORM-003**: The application form shall offer "Schedule a follow-up" with a date no earlier than the application date and +2 / +4 / +6 week shortcuts counted from the application date on the local calendar.
- [x] **APP-FORM-004**: Before submitting, the application form shall block the save with a message when a plant-required type has no plant (other than an edited entry whose plant was deleted, with its type unchanged), a follow-up is enabled without a date, or the follow-up date is before the application date.
- [x] **APP-FORM-005**: When submitting, the application form shall send empty fields as absent, and send FRAC classes only for a Fungicide with at least one class ticked.
- [x] **APP-FORM-006**: The application form's product picker shall load every shelf product and offer the non-archived products whose category accepts the application type, plus the currently linked product even when it is archived or no longer compatible, each labeled with its brand, "Other" when uncategorized, and its archived or non-In-stock state.
- [x] **APP-FORM-007**: The application form's product picker shall offer "— none —" (clearing the link and the name), each offered product, and "Not in inventory — type a name…" (revealing a free-text name); when the shelf is empty or cannot be loaded, it shall show only the free-text name.
- [x] **APP-FORM-008**: When the owner picks a shelf product in the application form, the form shall pre-fill the product name, N-P-K, FRAC classes, and the rate when the product has a recorded label rate, leaving every pre-filled field editable.
- [x] **APP-FORM-009**: When an application saves and the shelf product picked in that form session is marked Out, the form shall ask "<name> is marked Out in your inventory. Mark it In stock?" and on yes mark it In stock; the form shall change no other stock state.
- [x] **APP-FORM-010**: When the owner changes the type in the application form and the linked shelf product's category does not accept the new type (Product inventory's `categoriesFor`), the form shall drop the product link, keeping the product name as free text.
- [x] **APP-FORM-011**: If marking the product In stock fails after the application form's save succeeded, then the form shall treat the entry as saved (close the form and refresh the list) and report the stock failure on its own, so the entry is never submitted twice.

## Settings page

- [x] **APP-SET-001**: The Settings page shall edit the lawn profile's name (required), grass type, USDA zone (required), soil type ("Not specified" clears it), lawn size (empty clears it) and irrigation type ("Not specified" clears it), saving with one Save Profile button.
- [x] **APP-SET-002**: When the profile saves, the Settings page shall show "Profile saved successfully!" for 3 seconds and show the profile's created and last-updated times.
- [x] **APP-SET-003**: While no lawn profile exists, the Settings page shall show the empty profile form with a note that saving it creates the lawn profile, instead of a load error, and Save Profile shall create the profile through `PUT /api/v1/profile`.
