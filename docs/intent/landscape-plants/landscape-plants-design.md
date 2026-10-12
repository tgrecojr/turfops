---
parent: high-level-design
prefix: PLANT
---

# Landscape Plants

## Context and Design Philosophy

The lawn owner also keeps shrubs, trees, perennials and other landscape plants around the turf. They want the same "what do I do this week" help for those plants without having to research each one. Landscape Plants is the component that delivers it. The owner names a plant, and an LLM (through OpenRouter) writes a homeowner-level, year-round **care plan**: a list of maintenance **tasks**, each with a calendar **window**. The plan is cached in Postgres. From then on, deterministic code turns the cached windows into recommendations and seasonal-plan activities. It checks each window against today's date, the application log and the weather.

Three principles shape the component:

- **The LLM writes the plan. Code decides when to surface it.** The HLD rules out LLM-authored recommendations. Here the LLM supplies only the plan content (task type, window, frequency, description, severity). Which tasks appear today, whether one is done, and whether the weather is already watering are pure functions of the cached plan, the log and the environmental summary (`backend/src/logic/plant_maintenance.rs`, "Pure functions — no IO").
- **The application log is the memory.** A task is complete when a matching plant-scoped application is logged against that plant inside the task's window. There is no separate "done" checkbox. Watering is the one exception: it is a "during dry spells" reminder, not a one-time job, so no log entry completes it and the owner answers it with Mark Addressed.
- **OpenRouter is optional.** Without a key, adding a plant or regenerating its plan returns 503. Plants already saved keep working: their windows, recommendations and plan overlay never call the LLM again.

This component also owns the **shared OpenRouter client** (`backend/src/datasources/openrouter.rs`): transport, attribution headers, strict-JSON-schema completion and error mapping. Product inventory owns its own prompt and schema (`datasources/openrouter/product.rs`) and calls the same `complete_json`.

## Data Model

`backend/src/models/plant.rs`

| Type | Shape | Notes |
|------|-------|-------|
| `Plant` | id, lawn_profile_id, common_name, scientific_name?, plant_type, location?, planting_date?, notes?, maintenance_plan, plan_generated_at, plan_model, created_at, updated_at | Owner-entered metadata plus the cached plan and which model wrote it. |
| `PlantType` | Shrub · Tree · Perennial · Annual · Vine · Groundcover · Other | `FromStr` is case-insensitive and accepts "ground cover". |
| `PlantMaintenancePlan` | identified_name, scientific_name?, identification_confidence (High/Medium/Low), summary, tasks[], warnings[] | Stored as JSONB. `warnings` defaults to empty on read. |
| `MaintenanceTask` | task_type, window_start_month_day, window_end_month_day ("MM-DD"), frequency, description, severity, zone_note? | Windows are month-day strings so the plan repeats every year. |
| `TaskType` | Pruning · Fertilizing · Mulching · Watering · PestInspection · Deadheading · WinterProtection · Other | `as_str` gives display labels ("Pest Inspection", "Winter Protection"). |
| `TaskFrequency` | Once · Twice · Monthly · AsNeeded | Display-only: stored and shown on the page, read by no window, completion, recommendation or plan logic. One completing application completes a "Twice" or "Monthly" task for its whole window (PLANT-DATA-007). |

### Task → application types that complete it

`TaskType::matching_application_types` (`models/plant.rs:81`):

| Task | Completed by logging | Suggested type in the action text |
|------|---------------------|-----------------------------------|
| Pruning | Pruning | Pruning |
| Fertilizing | PlantFertilizer | Plant Fertilizer |
| Mulching | Mulching | Mulching |
| Deadheading | Deadheading | Deadheading |
| WinterProtection | WinterProtection | Winter Protection |
| PestInspection | Other | Other |
| Other | Other | Other |
| Watering | nothing | none; the action says to use Mark Addressed |

A plant-linked Other entry completes every PestInspection and Other task of that plant whose window (with its 30-day early allowance) contains its date. The log has no finer type for an inspection or a free-form job, and one note of "done" is what the owner would write for either. Watering completes from nothing: one logged watering says nothing about the next dry spell inside the same window (PLANT-WIN-006).

The plant-scoped application types, the `applications.plant_id` link, the plant-scope rules and the duplicate key all belong to **Applications**. This component reads `application_type`, `plant_id` and `application_date` from the log.

## Persistence

Migration `backend/src/db/migrations/20260425000000_plants.sql`:

- `plants` table: `BIGSERIAL` id, `lawn_profile_id` FK → `lawn_profiles` `ON DELETE CASCADE`, `plant_type` constrained by `chk_plant_type`, `maintenance_plan JSONB NOT NULL`, `plan_generated_at` and `plan_model NOT NULL`, timestamps defaulting to `NOW()`. Index on `lawn_profile_id`.
- The same migration adds `applications.plant_id` (FK → `plants` `ON DELETE SET NULL`) and widens the application-type check to include the plant types. Applications owns those columns from then on.

Queries live in `backend/src/db/plant_queries.rs`:

| Function | Behavior |
|----------|----------|
| `list_plants_for_profile` | All plants for a profile, newest first. |
| `get_plant` | By id only. Not scoped to a profile. |
| `create_plant` | Inserts and returns the id. `created_at` / `updated_at` come from DB defaults. |
| `update_plant_metadata` | Reads the row, merges the partial update, writes metadata, sets `updated_at = NOW()`. Never touches the plan. 404 if missing. |
| `update_plant_plan` | Replaces `maintenance_plan` and `plan_model` and sets `plan_generated_at` and `updated_at` to `NOW()`. |
| `delete_plant` | In one transaction, writes the plant's name into `applications.plant_name` on its entries (APP-LOG-009, owned by Applications), then `DELETE`s by id. Returns the rows deleted, so the handler answers 404 for an unknown id (PLANT-API-013). |

On read, an unknown `plant_type` string falls back to `Other` with a warning log. A `maintenance_plan` that does not deserialize is an `InvalidData` error for that row: reading that one plant by id returns 400, while listing skips the plant and logs a warning naming it, so one corrupt row cannot take down `GET /plants`, the recommendation feed or the seasonal plan (PLANT-DATA-006).

## API Endpoints

`backend/src/api/plants.rs`, routed in `backend/src/main.rs`.

| Method | Path | Request | Response | Errors |
|--------|------|---------|----------|--------|
| GET | `/api/v1/plants` | — | `Plant[]` for the default lawn profile | 404 no profile |
| POST | `/api/v1/plants` | `{common_name?, scientific_name?, plant_type, location?, planting_date? (YYYY-MM-DD), notes?}` | 201 `Plant` | 400 for an unknown type, a bad date, or neither name given (checked first); 503 OpenRouter not configured, disabled or failing; 404 no profile |
| GET | `/api/v1/plants/{id}` | — | `Plant` | 404 |
| PUT | `/api/v1/plants/{id}` | Partial: `common_name?`; nullable `scientific_name`, `location`, `planting_date`, `notes`; `plant_type?` | `Plant` | 400 bad type or date, 404 |
| DELETE | `/api/v1/plants/{id}` | — | 204 | 404 when no such plant |
| POST | `/api/v1/plants/{id}/refresh-plan` | — | `Plant` with the new plan | 503, 404 |

**Create flow.** The request is validated first (a name, a known plant type, a well-formed planting date), and only then is the OpenRouter client checked, so a malformed request gets 400 even when no key is configured (PLANT-API-004). A 400 is the owner's to fix and a 503 is the server's, and fixing the request first spares a second round trip once the assistant is available. The LLM input string is "common (scientific)" when both names are given, otherwise whichever one is non-empty. The prompt sends the profile's `usda_zone`, the owner-selected `plant_type` and the location. Planting date and notes are stored but not sent. After generation, an empty `common_name` falls back to the plan's `identified_name`, and an empty `scientific_name` falls back to the plan's. The plan is saved whatever its `identification_confidence`, including a Low-confidence plan with no tasks.

**Refresh flow** rebuilds the input from the stored names and location, generates a new plan and overwrites it in place. The task list can come back in a different order or length.

**PUT** edits metadata only. Changing names or type does not regenerate the plan.

## OpenRouter Client (shared)

`backend/src/datasources/openrouter.rs`, configured in `backend/src/config.rs`, built in `main.rs`, held as `AppState.openrouter: Option<Arc<OpenRouterClient>>`.

| Setting | Source | Default |
|---------|--------|---------|
| API key | `OPENROUTER_API_KEY`. Empty or whitespace means not configured, and no client is built. | — |
| Model | `OPENROUTER_MODEL` | `anthropic/claude-haiku-4.5`, OpenRouter's dotted model id, wherever a default is supplied (backend config and Docker Compose) (PLANT-AI-010) |
| Enabled | `OPENROUTER_ENABLED` is `true`, `1` or `yes` in any case (Deployment, DEPLOY-CONFIG-013) | true |
| Base URL | `OPENROUTER_BASE_URL` | `https://openrouter.ai/api/v1` |
| Connect / request timeout | constants | 5 s / 60 s |

`complete_json<T>(system_prompt, user_prompt, schema_name, schema, temperature)` is the single request path:

1. If the client is disabled, it returns `TurfOpsError::disabled(Source::PlanAssistant)` without calling out: a 503 saying the plan assistant is turned off and that `OPENROUTER_ENABLED=true` turns it on.
2. It POSTs `/chat/completions` with the system prompt marked `cache_control: ephemeral` (prompt caching), the user prompt, `response_format: json_schema` with `strict: true`, and the temperature.
3. Every request carries a bearer key, **`HTTP-Referer: https://github.com/tgrecojr/turfops`** and **`X-Title: TurfOps`**, so OpenRouter's console attributes the traffic to this app.
4. A transport error (formatted without the URL), a non-2xx status, an unparseable body, missing `choices[0].message.content`, or content that does not deserialize into `T` all map to `TurfOpsError::unavailable(Source::PlanAssistant, detail)`, which the API returns as **503** "The plan assistant couldn't answer just now. Try again in a few minutes." The detail (upstream status and body, or the raw model content) is logged at WARN and never reaches the response. Product endpoints re-word the same errors for products (`for_products()`, owned by Product inventory).

The `Debug` impl for `OpenRouterConfig` redacts the key. Startup logs the configured model at INFO.

### Plant prompt and schema

`backend/src/datasources/openrouter/plant.rs`. The temperature is 0.2.

- **System prompt.** It writes for a homeowner, not a professional, and covers:
  - Identification: if the name is ambiguous, pick the most common residential variety and lower the confidence.
  - Tasks: homeowner-level only, nothing that needs a licensed applicator or heavy equipment.
  - Windows: every task gets an MM-DD window tuned to the zone, with one task per window when a job repeats.
  - Descriptions: short, and say why.
  - Warnings: only homeowner-relevant ones (toxicity, invasiveness, thorns, sap).
  - Unidentifiable plants: return Low confidence and no tasks.

  **Amounts in plan text.** Task descriptions reach the owner verbatim, as recommendation and plan-activity descriptions. They may carry agronomic amounts, such as a mulch depth or an amount of a nutrient, because those are part of the care advice. A nutrient amount may be expressed as an amount of a fertilizer or amendment ("a cup of 10-10-10 per shrub"), but not of a material that is also a pesticide active. They must never state a product's label rate or any rate for a pesticide (fungicide, herbicide, insecticide, miticide); the label governs product use. The prompt does not yet say this (see Deferred).
- **Strict schema** (`plan_json_schema`). Every field is required, including the nullable `scientific_name` and `zone_note`, and `additionalProperties: false`. Enums mirror `TaskType`, `TaskFrequency`, `IdentificationConfidence` and `Severity` (Info / Advisory / Warning / Critical). Both window strings must match `^(0[1-9]|1[0-2])-(0[1-9]|[12][0-9]|3[01])$`. The pattern allows impossible days such as 02-30, which the window logic clamps.

## Maintenance Windows

`backend/src/logic/plant_maintenance.rs`

**Parsing.** `parse_month_day(mmdd, year)` gives an exact date. If the day does not exist in that month, it falls back to the last valid day, so 02-30 becomes Feb 28 and 02-29 becomes Feb 28 in non-leap years. Malformed strings return `None`, and the task is skipped silently.

**Resolving a window.** `resolve_window(task, year)` returns the window that opens in `year`. When the end month-day falls before the start, the window wraps the new year and its end moves to `year + 1`.

**The window in force today.** `window_on(task, today)` first checks the window that opened last year and still has today on or before its end. If that one is not open, it uses this year's window. This keeps a December → February window in force through January and February, instead of jumping to next December's window on Jan 1. The current-year seasonal plan and the feed both resolve windows this way.

**Lead-in.** A recommendation fires from 7 days before the window opens (`WINDOW_LEAD_DAYS`) through the window's last day. When neither last year's nor this year's window is open or in its lead-in, the window that opens next year is checked too, so a window opening Jan 3 gets its lead-in from Dec 27 (PLANT-REC-008). Its recommendation carries next year's window, id and completion check.

**Completion** (`task_completed`). A task counts as done for the window `(start, end)` when the log has an application that meets all of these:

- it is linked to this plant (`plant_id`);
- its type is in the task's matching types;
- `start − 30 d ≤ date ≤ end` (`EARLY_COMPLETION_DAYS`);
- its date is on or before today.

Completion is per window. A spring pruning does not complete a fall pruning task, and a December pruning completes a December → February window on both sides of the new year. A Watering task has no matching types and never completes from the log.

## Recommendations

`generate_plant_maintenance_recommendations(plants, applications, env, today)` is one of the sources in the **Recommendation feed** (`api/recommendation_feed.rs::all`). The feed passes up to 1000 recent applications, the current `EnvironmentalSummary` and `today`, the lawn's local date from the app's `Clock` (`state.clock.today()`, resolved from `TZ`, `api/recommendation_feed.rs:54`). Every window, lead-in and completion check compares `NaiveDate`s against that one date; nothing in this component derives a date from UTC. For each plant and each task:

1. Resolve `window_on`. Skip the task if today is before the lead-in or after the end.
2. Skip it if the task is complete for this window.
3. Skip a Watering task while the weather is watering (see below).
4. Otherwise emit a `Recommendation` with:

| Field | Value |
|-------|-------|
| id | `plant_{plant_id}_{task_type}_{window start}`: the task type in snake_case and the window's start as YYYY-MM-DD in the year it opened, e.g. `plant_12_pruning_2026-12-01`. It never uses the task's position in the plan, so a regenerated plan that reorders its tasks leaves each saved answer on its own task. The year comes from the window's start, so the id stays the same across Jan 1 and an answer given in December still applies in January (PLANT-REC-002). |
| category | `PlantMaintenance` |
| severity | The task's LLM-assigned severity |
| title | "{common name} — {Task label} ({plant type})" |
| description | The task's LLM description |
| explanation | The window dates, with a note that this is a zone-tuned homeowner guideline |
| data points | Plant (Manual) · Window (OpenRouter) · Scientific Name and Zone Note when present (OpenRouter) |
| action | Every task but Watering: do the task within the window, then log it under Applications with the type that completes it, linked to the plant, so the reminder clears. Watering: water during dry spells and use Mark Addressed once watered, since logging does not clear it (PLANT-REC-005). |
| need | Fertilizing only: `ProductNeed("plant fertilizer", Fertilizer or Supplement)`, required (not optional) |

**Product inventory** evaluates the need: it enriches the recommendation with on-hand status and may add it to the shopping list. The feed applies the owner's addressed and dismissed answers afterwards. Plant tasks fall in the feed's seasonal answer episode of 120 days, except Watering, whose answer lasts the 7-day weather episode because it reports a weather condition (FEED-STATE-010, owned by the Recommendation feed; see Deferred 4).

### Watering hold-back

`backend/src/logic/plant_maintenance/watering.rs`. A plant Watering task means "water during dry spells" across its window. `WaterSignals::from_env` reads three signals:

| Signal | Source | Holds the reminder back when |
|--------|--------|------------------------------|
| Forecast rain | Sum of `precipitation_mm` over OpenWeatherMap points with timestamp ≤ now + 48 h (`next_hours`). The first slot is the current weather, so rain falling now counts. | ≥ `PRECIP_TRACE_MM` = 2.5 mm (≈ 0.1") |
| Recent rain | Lake `precipitation_7day_total_mm` | ≥ `PRECIP_HEAVY_7DAY_MM` = 25 mm (≈ 1") |
| Soil moisture | Station `primary_soil_moisture()` (10 cm, else 5 cm, else 20 cm) | ≥ `SOIL_MOISTURE_ADEQUATE` = 0.20 |

The thresholds are the shared constants in `logic/rules/thresholds.rs` (owned by **Agronomic rules**). They are checked in the order shown, and the first one met supplies the `wet_reason` text. A missing signal never holds the reminder back, so with no weather data at all the reminder still fires. When the reminder does fire, `annotate` adds "Rain last 7 d", "Rain forecast" and "Soil Moisture" data points for whichever signals are known. The hold-back applies only to the feed. The seasonal-plan activity for a Watering task is unaffected.

**Accepted limit: the reminder can return for up to a day after a shower.** The lake lags about a day, so a few hours after rain ends with none forecast, the 7-day total and the station's soil moisture have not yet caught up and no signal holds the reminder back. This is accepted rather than fixed: the forecast's first slot already covers rain falling now, and a day-old dry-spell reminder costs the owner a glance, while a second rain source would add a dependency for a one-day gap.

## Seasonal Plan and Calendar Overlay

`build_plant_activities(plants, applications, year, today)` is called by **Seasonal plan** (`api/seasonal_plan.rs`). Seasonal plan passes `today` as the server's local date (`api/seasonal_plan.rs:138`) and the applications dated from Oct 1 of `year − 1` through the last day of the latest window judged for `year`. That end reaches into `year + 1` when a window wraps the new year, so both a December completion of a window that wraps into `year` and a January completion of a window that opened in `year` and wraps out of it are visible (PLANT-PLAN-005). This component emits one `PlannedActivity` per task:

- **Window.** When `year` is the current year, the window comes from `window_on(task, today)`. For any other year it is the window that opens in that year.
- **Status.** Completed if `task_completed`; otherwise Missed if today is after the end, Active if today is on or after the start, and Upcoming before that. There is no lead-in here.
- **id** The same id the task's recommendation carries for the window shown (task type and the window's start in the year it opened), so in January a window that opened last December keeps last year's date and the plan, the Calendar and the feed name one task one way (PLANT-PLAN-003). **name** "{common name} — {Task label}". **category** `"Plant Maintenance"`. **confidence** Medium. No historical bounds and no soil trigger. `notes` is the zone note. `rate` is always `None`.
- **product_suggestions**. A fixed generic suggestion for Fertilizing (a balanced slow-release granular, with an example analysis) and for Mulching (shredded hardwood or pine bark, with a 2–3 in depth). The depth is an agronomic amount and intended; neither suggestion names a product or a rate. Other tasks have none.

The Calendar and Seasonal Plan pages render these activities. The Calendar recognizes the `"Plant Maintenance"` category string, draws those activities as thinner bars, and lists them in their own subsection. That category string is the contract.

## Failure and Degradation

| Condition | Effect |
|-----------|--------|
| No `OPENROUTER_API_KEY` | Add and refresh return 503 `not_configured(PlanAssistant)`: the plan assistant isn't configured, set `OPENROUTER_API_KEY` and restart. List, get, update, delete, the feed and the plan overlay work normally. |
| `OPENROUTER_ENABLED=false` | The client exists, but add and refresh return 503 saying the plan assistant is turned off and to set `OPENROUTER_ENABLED=true`. |
| OpenRouter error, timeout or schema mismatch | 503 saying the plan assistant couldn't answer and to try again later; the upstream detail is logged. Nothing is written, and on refresh the old plan is kept. |
| Lake or forecast unavailable | Watering reminders are not held back. All other tasks are unaffected. |
| Malformed window string in a stored plan | That task is skipped in both the feed and the plan. |
| Undeserializable stored plan | That plant is skipped, with a warning naming it, by `GET /plants`, the feed and the seasonal plan; every other plant keeps working. Reading that plant by id returns 400. |
| Plant deleted | Its applications keep their rows, with `plant_id` set to NULL and the plant's name in `plant_name`, so they stay landscape history (Applications). Its recommendations stop. Stored answers keyed by its ids remain and are never matched again. |

## Landscape Page

`frontend/src/pages/Landscape.tsx`, route `/landscape`, nav label "Landscape".

- **Header.** "+ Add Plant" toggles the add form. An intro paragraph says the windows surface in the Calendar, the Seasonal Plan and Recommendations.
- **Add form.** One name field, plant type (defaults to Shrub), location, planting date and notes. A client-side heuristic decides which name field to send: input that looks like "Genus species" (capitalized first word, at least two words) goes as `scientific_name`, anything else as `common_name`. The submit button reads "Generating plan…" while it waits, and a hint warns that generation takes 10–30 s. On success the new plant is appended and expanded.
- **List.** Plants are sorted by common name, case-insensitive. Each row is a collapsible button (`aria-expanded`) showing name, scientific name, type, location, task count, the collapsed summary, and an identification-confidence badge. The badge's text always includes the level, so color is never the only cue.
- **Expanded row.** Summary, planting date, notes when present, warnings, then the task list (type, window as "Mon D – Mon D", frequency, description, zone note). A footer shows when and by which model the plan was generated, with "Regenerate plan" and "Delete" (the latter confirmed with `confirm()`).
- **Edit.** The owner can edit a plant's common name, scientific name, plant type, location, planting date and notes. Saving calls `PUT /plants/{id}` and never regenerates the plan; a changed name or type takes effect in the plan only on "Regenerate plan" (PLANT-UI-012, PLANT-UI-013).
- **Errors.** Any failure (load, add, regenerate, edit, delete) sets the page-level error to `describeError(e)` (Web UI): the server's plain-language message (a 503 names the plan assistant), "There is no lawn profile yet. Set one up in Settings…" when loading finds no lawn profile, and "That took too long to answer. Try again." when generation outlasts the 60 s client timeout. A status code or JSON body is never shown.

The API client (`frontend/src/api/client.ts`) exports `updatePlant` for the edit form and `getPlant`. The Applications form's plant picker belongs to Applications. **MCP server** exposes a read-only `plants` tool that returns `GET /plants`.

## Decisions & Alternatives

| Decision | Chosen | Alternatives Considered | Rationale |
|----------|--------|------------------------|-----------|
| Who writes plant care advice | An LLM writes the plan once; deterministic code surfaces it | LLM-authored recommendations; a curated plant database | The HLD decision: the LLM supplies plant care plans, while recommendations come from deterministic, reproducible code. |
| Plan storage | JSONB cached on the plant, regenerated only on request | Generate on every view; normalized task tables | Keeps the idle app free of external calls and the plan stable between views. The plan is a document the LLM returns whole. [inferred] |
| Window format | "MM-DD" strings that repeat yearly | Absolute dates; month-only ranges | One plan serves every year (code comment: "so they apply year-over-year"). [inferred] |
| LLM output contract | OpenRouter strict `json_schema`, all fields required, enums mirroring Rust types | Free text parsed afterwards; JSON mode without a schema | Malformed output fails at the gateway or at serde instead of producing half-valid tasks. [inferred] |
| Windows that wrap the new year | `window_on` prefers last year's still-open window; recommendation and plan-activity ids both carry the window's start in the year it opened; the lead-in also looks at next year's window | Resolve only by calendar year; plan ids by requested year | A dormant-season task must not vanish on Jan 1 with weeks left, an answer given in December must still hold in January, the plan and the feed must name a task the same way, and a window opening in early January deserves the same week's notice as any other. |
| Task identity in ids | Task type plus the window's start date | The task's index in the plan list; a hash of the description | The index moves when a regenerated plan reorders tasks, carrying saved answers to the wrong task. Type and start survive regeneration whenever the task itself survives, and they are what the owner recognises as "the same job". Two tasks of one plant with the same type and start are one job by the prompt's one-task-per-window rule. |
| Which tasks the log completes | Pruning, PlantFertilizer, Mulching, Deadheading, WinterProtection by their own type; PestInspection and Other by Other; Watering by nothing | No log completion for PestInspection and Other; an Other entry also completing Watering | An inspection or a free-form job is a one-time act the owner records as Other. Watering is a recurring dry-spell response: a logged watering would silence the whole window after one dry week, so the owner answers it with Mark Addressed, which lasts one weather episode. |
| Task frequency | Display-only | Require that many completions per window | The owner reads the frequency as guidance; counting completions would demand a log entry per watering or inspection and still mis-time "AsNeeded". |
| A stored plan that cannot be read | Skip that plant with a warning in every listing | Fail the request (400); drop the bad tasks only | One corrupt row must not take down the plant list, the feed and the seasonal plan for every other plant (HLD graceful degradation). |
| Completion scope | Per window, counting from 30 days before it opens | Any matching log within the last year | Owner decision: April's feeding must not complete September's. |
| Completion signal | A matching plant-linked application in the log | A "done" toggle on the task | HLD: the application log is the memory of the lawn, and the owner should not keep a second record. |
| Watering reminder | Held back by forecast, 7-day rain or soil moisture, using the lawn's shared thresholds; no data keeps the reminder | Always show inside the window; a separate plant threshold set | A dry-spell reminder is noise while it rains. Shared constants keep the plant and lawn advice consistent, and a missing signal behaves like every other rule (degrade to showing). [inferred] |
| Fertilizing product need | Fertilizer or Supplement, required | Fertilizer only; optional need | Plant feedings are often specialty products such as acidifiers or micronutrients, and a due feeding with nothing on the shelf belongs on the shopping list. [inferred] |
| Recommendation severity | The task's LLM-assigned severity | A fixed severity for plant tasks | Lets a time-critical task (bloom-sensitive pruning) outrank routine care. [inferred] |
| Adding a plant without OpenRouter | Refused with 503 | Save metadata with an empty plan (as products allow) | A plant without a plan produces no windows, which is the whole point of the feature. [inferred] |
| OpenRouter attribution | `HTTP-Referer` and `X-Title` on every request | No attribution headers | Owner decision: LLM spend is tracked per app on the OpenRouter console, and unattributed calls cannot be told apart. |
| Client sharing | One client and `complete_json` path; each feature owns its prompt and schema module | A client per feature | One place for the key, timeouts, headers, prompt caching and error mapping. [inferred] |
| Prompt caching | System prompt marked `cache_control: ephemeral` | No caching | The system prompt is large and identical across calls. [inferred] |
| Amounts in plan text | Agronomic amounts (mulch depth, nutrient amounts) allowed; product label rates and pesticide rates never | No amounts at all; pass the LLM's text through unconstrained | HLD: agronomic amounts are part of the advice, while how much of a product to apply is set by its label, the legal authority, above all for pesticides. |
| Which "today" | The server's local date (`TZ`, set to the lawn's timezone) for windows, lead-in, completion and status | The UTC date; the viewer's browser date | HLD: every agronomic date is a lawn-local calendar day, and one configured clock gives every component the same day. A Dec 31 evening must not already be Jan 1. |

## Open Questions & Future Decisions

### Resolved
1. ✅ Windows that wrap the new year stay in force through January, and their recommendation id is stable across Jan 1 (owner decision).
2. ✅ Completion is scoped to one window, counting from 30 days before it opens (owner decision).
3. ✅ OpenRouter calls carry `HTTP-Referer` and `X-Title` attribution headers (owner decision).
4. ✅ Agronomic amounts in plan text are intended: the Mulching suggestion's depth (`logic/plant_maintenance.rs:238`) and any nutrient amount in a task description are care advice, not label rates.
5. ✅ "Today" is the server's local date. The feed passes the app `Clock`'s date (`state.clock.today()`, `api/recommendation_feed.rs:54`) and the seasonal plan passes `Local::now().date_naive()` (`api/seasonal_plan.rs:138`), both the lawn's local date under `TZ`. The only `Utc::now()` in this component stamps `plan_generated_at` (`api/plants.rs:92`), an instant rather than a date gate, and the watering forecast horizon (now + 48 h) compares instants. The Landscape page's planting-date input has no default, so the browser's date never enters a gate.
6. ✅ Plant Fertilizing tasks are not lawn feedings: plant fertilizer is outside the lawn nitrogen budget, so the nitrogen guard neither holds nor caps them.

### Deferred
1. **Accepted limit: watering reminder after a shower.** The lake lags about a day, so the reminder can return for up to a day after rain ends with none forecast. Accepted as a limit of the design, not a gap (see Watering hold-back).
2. **`TaskFrequency` is display-only** (PLANT-DATA-007). Intended, and the code agrees: one logged application completes a "Twice" or "Monthly" task for the whole window (`models/plant.rs:101`; no reader in `logic/`). Nothing to fix.
3. **Profile scoping.** Get, update, delete and refresh act by id without checking the profile (`db/plant_queries.rs:22`, `:74`, `:143`). This is harmless for a single-lawn app.
4. **Divergence: Watering answers last 120 days** (FEED-STATE-010, owned by the Recommendation feed). `PlantMaintenance` falls to the seasonal episode (`models/recommendation_state.rs:46`), so answering a dry-spell reminder hides it for a whole season. Intent: the 7-day weather episode.
5. **Attribution constants are hardcoded.** The owner's guidance prefers taking the app name from config or `Cargo.toml`.
6. **Size limits.** `pages/Landscape.tsx` is 683 lines, over the 300-line limit. Functions over 50 lines: `generate_plant_maintenance_recommendations` (88), `build_plant_activities` (65), `api::plants::create_plant` (82), `OpenRouterClient::complete_json` (85).
7. **Doc-precision gap.** The thresholds are 2.5 mm and 25 mm, which are about 0.098" and 0.984". The docs say "≥ 0.1"" and "≥ 1"".

## References

- `docs/high-level-design.md`: the Landscape plants component, the LLM non-goal and decision, and the graceful-degradation tenet.
- `docs/agronomy-methods.md`: the plant watering dry-spell thresholds row.
- Code: `backend/src/models/plant.rs`, `backend/src/api/plants.rs`, `backend/src/db/plant_queries.rs`, `backend/src/db/migrations/20260425000000_plants.sql`, `backend/src/datasources/openrouter.rs`, `backend/src/datasources/openrouter/plant.rs`, `backend/src/logic/plant_maintenance.rs`, `backend/src/logic/plant_maintenance/watering.rs`, `backend/src/logic/plant_maintenance/tests.rs`, `backend/src/config.rs`, `frontend/src/pages/Landscape.tsx`.
- Boundaries: Recommendation feed (`api/recommendation_feed.rs`), Seasonal plan (`api/seasonal_plan.rs`), Product inventory (`ProductNeed`, `datasources/openrouter/product.rs`), Applications (plant-scoped types, `plant_id`), Agronomic rules (`logic/rules/thresholds.rs`), MCP server (`mcp/tools/shelf.rs`).
