# Landscape Plants — EARS Specs

Design: [landscape-plants-design.md](landscape-plants-design.md). Prefix `PLANT`. Facets: `DATA` (plant model and stored plan), `API` (plants endpoints), `AI` (shared OpenRouter client), `PROMPT` (plant care-plan prompt and schema), `WIN` (maintenance windows and completion), `REC` (plant task recommendations), `WATER` (watering hold-back), `PLAN` (seasonal-plan activities), `UI` (Landscape page).

A **care plan** is the LLM-written list of maintenance **tasks** cached on a plant; each task has a type, a yearly **window** given as "MM-DD" start and end strings, a frequency, a description, a severity and an optional zone note. "Today" below is the lawn's local date from the app clock (governed by `TZ`). A **plant-linked application** is a log entry whose `plant_id` is that plant (plant-scoped application types and the link are owned by Applications).

## Plant model and stored plan

- [x] **PLANT-DATA-001**: The system shall store each landscape plant with its lawn profile, common name, optional scientific name, plant type, optional location, optional planting date, optional notes, the cached care plan, the plan's generation time and the model that wrote it.
- [x] **PLANT-DATA-002**: The system shall accept the plant types Shrub, Tree, Perennial, Annual, Vine, Groundcover and Other, parsed case-insensitively, with "ground cover" accepted as Groundcover.
- [x] **PLANT-DATA-003**: If a stored plant row carries an unknown plant-type string, then the system shall read it as Other and log a warning.
- [x] **PLANT-DATA-004**: The system shall store a plant's care plan as one document holding the identified name, optional scientific name, identification confidence (High / Medium / Low), a summary, the task list and a warnings list, and shall read a stored plan without a warnings list as having none.
- [x] **PLANT-DATA-005**: The system shall store each care-plan task's window as "MM-DD" start and end strings, so one plan applies every year.
- [x] **PLANT-DATA-006**: If a stored plant's care plan cannot be read, then the system shall skip that plant and log a warning naming it when listing plants for `GET /api/v1/plants`, the recommendation feed and the seasonal plan, instead of failing the whole request.
- [x] **PLANT-DATA-007**: The system shall treat a care-plan task's frequency (Once / Twice / Monthly / AsNeeded) as display-only: no window, completion, recommendation or plan logic reads it, so one completing application completes a task for its whole window whatever its frequency.

## Plants endpoints

- [x] **PLANT-API-001**: When `GET /api/v1/plants` is called, the system shall return every plant of the default lawn profile, newest first, or 404 when no lawn profile exists.
- [x] **PLANT-API-002**: When `POST /api/v1/plants` is called, the system shall generate a care plan through OpenRouter from the plant's name, the lawn profile's USDA zone, the owner-selected plant type and the location, save the plant with that plan, the configured model id and the current time, and respond 201 with the saved plant.
- [x] **PLANT-API-003**: When `POST /api/v1/plants` gives both a non-empty common name and scientific name, the system shall send the LLM the name as "common (scientific)"; otherwise it shall send whichever one is non-empty.
- [x] **PLANT-API-004**: If a `POST /api/v1/plants` body has neither a non-empty common name nor a non-empty scientific name, an unknown plant type, or a planting date that is not YYYY-MM-DD, then the system shall respond 400 with a message naming the bad input, whether or not an OpenRouter client is configured (validation runs before the OpenRouter check), and shall not call the LLM.
- [x] **PLANT-API-005**: When `POST /api/v1/plants` saves a plant whose request left the common name or scientific name empty, the system shall fill it from the care plan's identified name or scientific name.
- [x] **PLANT-API-006**: When `POST /api/v1/plants` receives a care plan, the system shall save it whatever its identification confidence, including a Low-confidence plan with no tasks.
- [x] **PLANT-API-007**: If care-plan generation fails during `POST /api/v1/plants` or `POST /api/v1/plants/:id/refresh-plan`, then the system shall respond 503 and write nothing, leaving any existing plan in place.
- [x] **PLANT-API-008**: When `GET /api/v1/plants/:id` is called, the system shall return that plant, or 404 when no plant has that id.
- [x] **PLANT-API-009**: When `PUT /api/v1/plants/:id` is called, the system shall update only the metadata fields the body supplies (a JSON `null` clears scientific name, location, planting date or notes), set `updated_at` to now, leave the care plan untouched without regenerating it, and return the updated plant.
- [x] **PLANT-API-010**: If `PUT /api/v1/plants/:id` names a plant that does not exist, then the system shall respond 404; if its plant type is unknown or its planting date is not YYYY-MM-DD, then it shall respond 400.
- [x] **PLANT-API-011**: When `DELETE /api/v1/plants/:id` deletes a plant, the system shall respond 204, stop emitting that plant's task recommendations and plan activities, and keep its plant-linked applications in the log with their plant link cleared.
- [x] **PLANT-API-012**: When `POST /api/v1/plants/:id/refresh-plan` is called, the system shall regenerate the care plan from the plant's stored names, plant type and location and the profile's USDA zone, replace the stored plan and model id, set `plan_generated_at` and `updated_at` to now, and return the updated plant, or 404 when no plant has that id.
- [x] **PLANT-API-013**: If `DELETE /api/v1/plants/:id` names a plant that does not exist, then the system shall respond 404.

## Shared OpenRouter client

- [x] **PLANT-AI-001**: The system shall build the shared OpenRouter client only when `OPENROUTER_API_KEY` is set to a non-blank value, taking the model from `OPENROUTER_MODEL`, the base URL from `OPENROUTER_BASE_URL` (default `https://openrouter.ai/api/v1`) and the enabled flag from `OPENROUTER_ENABLED` (read as DEPLOY-CONFIG-013 defines: `true`, `1` or `yes` in any case is on, unset is on).
- [x] **PLANT-AI-002**: The shared OpenRouter client shall send every completion request with the bearer API key and the attribution headers `HTTP-Referer: https://github.com/tgrecojr/turfops` and `X-Title: TurfOps`, so the owner's OpenRouter console attributes the spend to this app.
- [x] **PLANT-AI-003**: The shared OpenRouter client shall send every completion with the system prompt marked for prompt caching (`cache_control: ephemeral`) and a `json_schema` response format with `strict: true`, and shall deserialize the returned content into the caller's type.
- [x] **PLANT-AI-004**: The shared OpenRouter client shall use a 5 s connect timeout and a 60 s request timeout.
- [x] **PLANT-AI-005**: While `OPENROUTER_ENABLED` is off, the shared OpenRouter client shall refuse every completion without calling OpenRouter, and the caller shall answer 503.
- [x] **PLANT-AI-006**: If no OpenRouter client is configured, then a valid `POST /api/v1/plants` (one that passes PLANT-API-004) and `POST /api/v1/plants/:id/refresh-plan` shall respond 503 with a message saying OpenRouter is not configured and that `OPENROUTER_API_KEY` must be set, while listing, reading, editing and deleting plants and their recommendations and plan activities keep working.
- [x] **PLANT-AI-007**: If an OpenRouter completion fails (connection error, timeout, non-2xx status, unparseable body, missing content, or content that does not match the schema), then the shared OpenRouter client shall log the upstream status, response body or raw model content and return a plain-language message that says the plan assistant could not answer and to try again later, never including the upstream body, the raw model content or the request URL.
- [x] **PLANT-AI-008**: While `OPENROUTER_ENABLED` is off, the 503 message from the shared OpenRouter client shall say the plan assistant is turned off and that `OPENROUTER_ENABLED` must be "true" to use it.
- [x] **PLANT-AI-009**: The system shall never write the OpenRouter API key to logs or debug output.
- [x] **PLANT-AI-010**: Where `OPENROUTER_MODEL` is unset, the shared OpenRouter client shall use the model id `anthropic/claude-haiku-4.5` (OpenRouter's dotted form), and every place that supplies a default OpenRouter model id (backend config and Docker Compose) shall use that same id.

## Care-plan prompt and schema

- [x] **PLANT-PROMPT-001**: When generating a plant care plan, the system shall send the user prompt with the plant name input, the USDA hardiness zone, the owner-reported plant type and the location (or "unspecified"), and shall not send the planting date or notes.
- [x] **PLANT-PROMPT-002**: The plant care-plan system prompt shall ask for a year-round, homeowner-level plan with nothing that needs a certified applicator license or equipment beyond hand pruners, loppers and a bow rake.
- [x] **PLANT-PROMPT-003**: The plant care-plan system prompt shall ask the model, when a name is ambiguous, to pick the most common residential variety and lower the identification confidence, and, when it cannot identify the plant at all, to return Low confidence, a summary saying why, and no tasks.
- [x] **PLANT-PROMPT-004**: The plant care-plan system prompt shall ask for an "MM-DD" window on every task, tuned to the given zone, with one task per window when a job repeats within the year.
- [x] **PLANT-PROMPT-005**: The plant care-plan system prompt shall ask for short task descriptions (1–3 sentences) that say why, and for warnings only when they matter to a homeowner (pet toxicity, invasiveness, thorns, irritant sap).
- [x] **PLANT-PROMPT-006**: The plant care-plan system prompt shall tell the model that task descriptions may give agronomic amounts (such as mulch depth or an amount of a nutrient) but must never state a product's label rate or any rate or amount for a pesticide (fungicide, herbicide, insecticide, miticide), because the label governs product use.
- [x] **PLANT-PROMPT-007**: The plant care-plan system prompt shall tell the model that a nutrient amount may be expressed as an amount of a fertilizer or soil-amendment material (for example a cup of 10-10-10 per shrub) but never as an amount of a material that is also a pesticide active (copper sulfate, for example, is given only as the nutrient amount).
- [x] **PLANT-PROMPT-008**: The plant care-plan JSON schema shall require every field (including the nullable scientific name and zone note), forbid additional properties, restrict task type, frequency, identification confidence and severity to the enums the backend's types define, and require both window strings to match `^(0[1-9]|1[0-2])-(0[1-9]|[12][0-9]|3[01])$`.
- [x] **PLANT-PROMPT-009**: The system shall request plant care plans at temperature 0.2.

## Maintenance windows and completion

- [x] **PLANT-WIN-001**: When resolving a care-plan task window for a year, the system shall turn each "MM-DD" string into a date in that year, using the month's last valid day when the day does not exist (02-30 and, outside leap years, 02-29 become Feb 28).
- [x] **PLANT-WIN-002**: If a care-plan task's window string is malformed, then the system shall skip that task in both the recommendation feed and the seasonal plan without failing the rest.
- [x] **PLANT-WIN-003**: When a care-plan task's end month-day falls before its start month-day, the system shall treat the window as wrapping the new year, ending in the year after the one it opens in.
- [x] **PLANT-WIN-004**: When deciding which window of a care-plan task is in force on a given day, the system shall use the window that opened last year while that day is on or before its end, and otherwise the window that opens this year, so a December → February window stays in force through January and February.
- [x] **PLANT-WIN-005**: The system shall count a care-plan task as complete for a window only when a plant-linked application for that plant has a type that completes the task and is dated from 30 days before the window opens through the window's end, and not after today.
- [x] **PLANT-WIN-006**: The system shall treat a care-plan task as completed by these application types only: Pruning by Pruning, Fertilizing by PlantFertilizer, Mulching by Mulching, Deadheading by Deadheading, WinterProtection by WinterProtection, PestInspection by Other, and Other by Other; a Watering task is completed by no application type.
- [x] **PLANT-WIN-007**: The landscape-plant task logic shall be pure functions of the plants, the application log, the environmental summary and a "today" its caller supplies as the server's local date, performing no I/O and deriving no date from UTC.

## Plant task recommendations

- [x] **PLANT-REC-001**: When building the recommendation feed, the landscape-plant source shall emit one recommendation per care-plan task whose window in force today (PLANT-WIN-004) opens within the next 7 days or is open, from 7 days before it opens through its last day, unless the task is complete for that window (the lead-in into a window that opens next year is PLANT-REC-008).
- [x] **PLANT-REC-002**: The landscape-plant source shall give each task recommendation the id `plant_{plant_id}_{task_type}_{window start}`, with the task type in snake_case and the window start as YYYY-MM-DD in the year the window opened (e.g. `plant_12_pest_inspection_2026-03-15`), never the task's position in the plan, so regenerating a plan that reorders its tasks keeps each saved answer on its own task, and the id stays the same across Jan 1 for a window that wraps the new year.
- [x] **PLANT-REC-003**: The landscape-plant source shall give each task recommendation the category PlantMaintenance, the task's own severity, the title "{common name} — {task label} ({plant type})", and the task's description as its description.
- [x] **PLANT-REC-004**: The landscape-plant source shall give each task recommendation an explanation stating the window's dates and that it is a zone-tuned homeowner guideline to adjust for the microclimate, and data points for the plant, the window, and the scientific name and zone note when present.
- [x] **PLANT-REC-005**: The landscape-plant source shall give each task recommendation other than Watering an action telling the owner to do the task within the window and log it under Applications, linked to the plant, with the application type that completes it (PLANT-WIN-006; Other for PestInspection and Other tasks), and shall give a Watering task recommendation an action telling the owner to use Mark Addressed once the plant is watered, because logging does not clear it (its answer lasts the 7-day weather episode, FEED-STATE-010).
- [x] **PLANT-REC-006**: When the landscape-plant source emits a Fertilizing task recommendation, it shall declare a required "plant fertilizer" product need satisfied by a Fertilizer or Supplement product, with no nitrogen requirement.
- [x] **PLANT-REC-007**: The landscape-plant source shall never mark a task recommendation as a feeding (advice to apply lawn nitrogen now), so the nitrogen guard neither holds nor caps a plant Fertilizing task and adds no "N budget remaining" point to it.
- [x] **PLANT-REC-008**: When no window of a care-plan task is open or in its 7-day lead-in on a day under PLANT-WIN-004 and the task's window that opens next year opens within the next 7 days, the landscape-plant source shall emit that task's recommendation for next year's window, so a window opening in early January gets its lead-in in late December.

## Watering hold-back

- [x] **PLANT-WATER-001**: While forecast rain over the OpenWeatherMap points timed within the next 48 hours (the first point being the current weather) totals at least 2.5 mm, the landscape-plant source shall not emit Watering task recommendations.
- [x] **PLANT-WATER-002**: While the weather lake's 7-day precipitation total is at least 25 mm, the landscape-plant source shall not emit Watering task recommendations.
- [x] **PLANT-WATER-003**: While the station's current soil moisture (10 cm, else 5 cm, else 20 cm) is at least 0.20, the landscape-plant source shall not emit Watering task recommendations.
- [x] **PLANT-WATER-004**: The watering hold-back shall use the same rain and soil-moisture threshold constants as the lawn irrigation rule (owned by Agronomic rules), not a separate plant set.
- [x] **PLANT-WATER-005**: If a watering signal (forecast rain, 7-day rain or soil moisture) is unavailable, then the landscape-plant source shall treat it as not holding the reminder back, so with no weather data a Watering task recommendation still fires.
- [x] **PLANT-WATER-006**: When the landscape-plant source emits a Watering task recommendation, it shall add data points for whichever of 7-day rain, forecast rain and soil moisture are known, with rain shown in inches.
- [x] **PLANT-WATER-007**: The watering hold-back shall apply only to the recommendation feed; a Watering task's seasonal-plan activity shall not depend on the weather.

## Seasonal-plan activities

- [x] **PLANT-PLAN-001**: When the seasonal plan is built for a year, the landscape-plant logic shall add one activity per care-plan task, using the window in force today (PLANT-WIN-004) when the year is the current year and otherwise the window that opens in that year.
- [x] **PLANT-PLAN-002**: The landscape-plant logic shall give each plant activity the status Completed when the task is complete for that window (PLANT-WIN-005), otherwise Missed when today is after the window's end, Active when today is on or after its start, and Upcoming before that, with no lead-in.
- [x] **PLANT-PLAN-003**: The landscape-plant logic shall give each plant activity the same id its task's recommendation carries for the window the activity shows (PLANT-REC-002, built from the task type and the window's start in the year the window opened, so a window that opened last December keeps that year in January), the name "{common name} — {task label}", the category string "Plant Maintenance" (the contract the Calendar uses to draw plant bars), Medium confidence, no historical bounds or soil trigger, the zone note as its notes, and no rate.
- [x] **PLANT-PLAN-004**: The landscape-plant logic shall give Fertilizing activities the one generic suggestion "Balanced slow-release granular (e.g., 10-10-10)" and Mulching activities "Shredded hardwood or pine bark mulch, 2-3 in", naming no product and no rate, and other tasks no suggestions.
- [x] **PLANT-PLAN-005**: When `GET /api/v1/seasonal-plan` builds plant activities for a year, it shall supply the plant-linked log from Oct 1 of the previous year through the last day of the latest window it judges for that year (into the following year when a window wraps the new year), and today as the server's local date, so both a December completion of a window that wraps into the year and a January completion of a window that wraps out of it count.

## Landscape page

- [x] **PLANT-UI-001**: The web UI shall serve the Landscape page at `/landscape`, linked from the navigation as "Landscape".
- [x] **PLANT-UI-002**: The Landscape page shall show a "+ Add Plant" button that toggles the add form, and an intro saying the plan's windows appear in the Calendar, Seasonal Plan and Recommendations.
- [x] **PLANT-UI-003**: The Landscape page's add form shall offer one name field, a plant type (default Shrub), location, planting date and notes, and shall send the name as the scientific name when it looks like "Genus species" (a capitalized word followed by a lowercase word) and as the common name otherwise.
- [x] **PLANT-UI-004**: While the Landscape page's add request is in flight, the submit button shall read "Generating plan…", the form shall note that generation may take 10–30 seconds, and the request shall wait up to 60 s; regenerating a plan shall also wait up to 60 s.
- [x] **PLANT-UI-005**: When a plant is added on the Landscape page, the page shall close the form and show the new plant expanded.
- [x] **PLANT-UI-006**: The Landscape page shall list plants sorted by common name, case-insensitively, each as a collapsible row (`aria-expanded`) showing the name, scientific name, type, location, task count, the summary while collapsed, and an identification-confidence badge whose text includes the level.
- [x] **PLANT-UI-007**: When a plant row on the Landscape page is expanded, it shall show the summary, the planting date, the warnings, each task's type, window as "Mon D – Mon D", frequency, description and zone note, and a footer with the plan's generation date and model plus "Regenerate plan" and "Delete" buttons.
- [x] **PLANT-UI-008**: When the owner clicks "Delete" on the Landscape page, the page shall ask for confirmation before deleting the plant and removing its row.
- [x] **PLANT-UI-009**: While no plants exist, the Landscape page shall show "No plants yet. Click "+ Add Plant" to get started."
- [x] **PLANT-UI-010**: If a Landscape page request fails, then the page shall show a plain-language message from the server's `error` text, never the HTTP status code or raw JSON body: a 503 shall say the plan assistant (OpenRouter) is unavailable, and a timeout shall say generation took too long and to try again.
- [x] **PLANT-UI-011**: If loading plants on the Landscape page fails because no lawn profile exists (404), then the page shall tell the owner to create the lawn profile in Settings first.
- [x] **PLANT-UI-012**: The Landscape page shall let the owner edit a plant's common name, scientific name, plant type, location, planting date and notes, saving through `PUT /api/v1/plants/:id` without regenerating the care plan.
- [x] **PLANT-UI-013**: When a plant row on the Landscape page is expanded and the plant has notes, the page shall show the notes.
