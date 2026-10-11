---
parent: high-level-design
prefix: UI
---

# Web UI Shell & Conventions

## Context and Design Philosophy

The web UI is a React single-page application served by the same Rust binary as the API. This component owns everything the pages share and nothing any one feature owns: the entry point, the router and its route table, the navigation shell, the typed fetch wrapper over `/api/v1`, the shared TypeScript model mirror, shared styles and helpers, the build and lint tooling, and the backend's static-file serving. It also owns the cross-cutting presentation rules every feature page follows (the status palette, symbol-plus-label badges, table views beside charts, forecast styling, colour-vision-safe series colours).

Feature components own their pages and widgets. The Disease Risk page belongs to disease-risk (DIS), the Timing page to timing-windows (TIME), and so on (see *Route Ownership*). The shell renders them inside one layout and gives each one the same client, types and conventions, so the HLD goal "every surface gives the same answer" is not undercut by a page rendering the same state differently.

Three constraints shape the design:

- **One owner, one lawn, one browser.** No authentication, no global client-side store and no server-state cache library. Each page fetches what it shows when it mounts, which also preserves the HLD's "no external calls while idle": the backend only refreshes sources when a page asks.
- **The backend is the authority.** The UI computes no agronomy. It formats, sorts and colours what the API returns. Server enums arrive as their Rust variant names and are mapped to labels on the client.
- **Accessibility over decoration.** State is never carried by colour alone, and every chart has a table view.

## Entry Point and Error Containment

`frontend/index.html` titles the document "TurfOps", provides `#root` and loads `src/main.tsx`, which mounts `<App/>` under `React.StrictMode` and throws if `#root` is missing. `src/index.css` sets the global reset (border-box, system font stack, page background `#f7fafc`, ink `#1a202c`).

`src/App.tsx` wraps everything in a class `ErrorBoundary` (the only error boundary in the app). It catches render errors from any page, shows the error message, and offers "Return to Dashboard", which clears the error and does a full navigation to `/`. Data-fetch failures do **not** reach the boundary: every page catches its own fetch errors and renders `sharedStyles.error` inline.

Every page is a `React.lazy` import, so each page (and the chart library, which only some pages use) is a separate chunk. A single `<Suspense>` around `<Routes>` shows "Loading..." while a chunk loads.

## Route Table and Ownership

All routes are children of one layout route (`<Route element={<Layout/>}>`), so the sidebar persists across navigation. There are 14 routes plus a wildcard (`frontend/src/App.tsx`).

| Path | Page module | Nav label | Owning component |
|---|---|---|---|
| `/` (index) | `pages/Dashboard.tsx` | Dashboard | web-ui (composition); widgets owned by their features |
| `/applications` | `pages/Applications.tsx` | Applications | applications (APP) |
| `/inventory` | `pages/Inventory.tsx` | Inventory | product-inventory (INV) |
| `/landscape` | `pages/Landscape.tsx` | Landscape | landscape-plants (PLANT) |
| `/calendar` | `pages/Calendar.tsx` | Calendar | seasonal-plan (PLAN) |
| `/environmental` | `pages/Environmental.tsx` | Environmental | weather-data (WX-VIEW); current summary and force-refresh from data-sync (SYNC) |
| `/recommendations` | `pages/Recommendations.tsx` | Recommendations | recommendation-feed (FEED) |
| `/disease-risk` | `pages/DiseaseRisk.tsx` | Disease Risk | disease-risk (DIS) |
| `/disease-risk/:slug` | `pages/DiseaseRisk.tsx` | (deep link, no nav item) | disease-risk (DIS) |
| `/timing` | `pages/Timing.tsx` | Seed & Pre-Em Timing | timing-windows (TIME) |
| `/soil-tests` | `pages/SoilTests.tsx` | Soil Tests | soil-tests (SOIL) |
| `/seasonal-plan` | `pages/SeasonalPlan.tsx` | Seasonal Plan | seasonal-plan (PLAN) |
| `/settings` | `pages/Settings.tsx` | Settings | applications (APP) — lawn profile form |
| `*` | `NotFound` in `App.tsx` | — | web-ui |

The unknown-path page renders inside the layout and links back to the dashboard.

### Dashboard composition

The dashboard has no single feature owner, so the shell owns its composition (`pages/Dashboard.tsx`). It loads `getDashboard()` (required) in parallel with `getGdd`, `getNitrogenBudget`, `getSoilTempForecast`, `getDiseaseRisk` and `getTimingWindows`. Each of those is caught individually, so one failing feature affects only its widget (a widget that has shown data keeps it on a failed re-poll, FEED-UI-013). Only a failed `/dashboard` call shows the page error. While the tab is visible the dashboard re-polls every 30 s. It stops polling when `document.hidden` and refetches once when the tab becomes visible again. A new poll aborts the requests of a previous poll still in flight: the dashboard's `AbortController` signal is passed through the client to `fetch`, so a superseded poll stops on the network instead of running to completion with its results thrown away. The Environmental page (WX-VIEW) is the only other page that polls, with the same 30 s visibility-aware loop (`pages/Environmental.tsx:71-102`). A soil reading whose `soil_observed_at` is older than 72 h shows the stale-data banner, matching the backend's 72 h cutoff.

| Dashboard slot | Component | Owner |
|---|---|---|
| Gauges (soil temp, ambient, humidity, soil moisture) | `components/Gauge.tsx` + `gaugeConfigs.ts` | WX-VIEW (shared with the Environmental page) |
| Alert cards (top of feed) | `components/AlertCard.tsx` | FEED |
| Disease risk widget | `components/disease/DiseaseRiskWidget.tsx` | DIS |
| Timing widget | `components/timing/TimingWidget.tsx` | TIME |
| GDD widget | `components/GddWidget.tsx` | RULE |
| Nitrogen budget widget | `components/NitrogenBudgetWidget.tsx` | RULE |
| Soil-temp outlook widget | `components/SoilTempForecastWidget.tsx` | WX-OUTLOOK |
| Shopping list (compact) | `components/inventory/ShoppingListPanel.tsx` | INV |

## Navigation Shell

`components/Layout.tsx` is a fixed 220 px dark sidebar (`#1a202c`) with the "TurfOps" wordmark and a `NAV_ITEMS` list of 12 `NavLink`s, plus a scrolling `<main>` that renders the `<Outlet/>`. The active link gets a darker background, green text (`#68d391`) and bold weight, so the active state is carried by weight as well as colour. `end` is set only on `/`, so the Dashboard link is not active on every route. Nav order is the sidebar order in the table above (minus the `:slug` and wildcard routes). The nav has `aria-label="Main navigation"`.

On a narrow screen (phone width) the sidebar collapses: the page takes the full width and the twelve links open from a menu control that stays inside the "Main navigation" landmark, with the same active-link styling. The exact breakpoint is an implementation choice; the rule is that the 220 px sidebar never squeezes the page at phone width.

## API Client Contract

`frontend/src/api/client.ts` is the only module that calls `fetch`. Every endpoint function returns a typed promise through one private wrapper, `fetchJson<T>(url, init?, timeoutMs = 15_000)`.

### Wrapper behaviour

| Concern | Behaviour |
|---|---|
| Base URL | `BASE = "/api/v1"`, relative. Same-origin in production. In development the Vite server proxies `/api` to `http://localhost:3000`. |
| Headers | `Content-Type: application/json` on every call, GETs included. A caller's `headers` are merged with the defaults (a caller's header of the same name wins), never replace them. |
| Timeout | Every request gets its own `AbortController` timer at `timeoutMs`. A caller's `signal` is combined with it (either one aborts the request), so a caller can cancel a request but cannot remove its timeout. Only the timer's own abort reads as a timeout (plain message, UI-ERR-008); a caller's cancellation is the caller's to ignore. |
| Non-2xx | Reads the body as **text**, truncates to 200 chars, replaces `/path/file.(rs|js|ts):<line>` with `[internal]`, and throws `Error("<status>: <body>")`. |
| 204 or empty 2xx body | Resolves `undefined` (no content), never a JSON parse error. |
| 2xx with a body | `res.json()`, unvalidated, cast to `T`. |

The backend's error body is `{"error": "<message>"}` (`backend/src/error.rs`): 404 `NotFound`, 400 `InvalidData`, 503 `DataSourceUnavailable`, 409 for a unique violation (the application-log duplicate guard), and a generic 500 "An internal error occurred" for everything else. The client does not parse it. Pages show `e.message` verbatim, so the user sees, for example, `409: {"error":"An identical entry …"}`.

Per-call timeouts: 30 s for the seasonal plan and the shopping list (both re-evaluate expensive state); 60 s for `createPlant`, `refreshPlantPlan`, `createProduct` and `refreshProductProfile` (an OpenRouter round-trip). Everything else uses the 15 s default.

### Endpoint functions by owning component

| Owner | Client functions → endpoint |
|---|---|
| SYNC | `getHealth` → `GET /health` *(defined, not called by any page)* |
| web-ui | `getDashboard` → `GET /dashboard` |
| APP | `getProfile`, `updateProfile` → `GET/PUT /profile` (an empty `soil_type`/`irrigation_type` or `lawn_size_sqft: 0` clears the field) · `getApplications(type?, limit=50, offset=0)` · `createApplication` · `updateApplication` · `deleteApplication` · `getFracClasses(product?)` → `/frac-classes` |
| PLAN | `getCalendar(year?, month?)` → `/applications/calendar` · `getSeasonalPlan(year?)` |
| SYNC | `getEnvironmental` · `refreshEnvironmental` (POST) |
| WX-VIEW | `getHistorical("7d"\|"30d"\|"90d")` |
| WX-OUTLOOK | `getSoilTempForecast` |
| RULE | `getGdd(year?)` · `getNitrogenBudget(year?)` |
| FEED | `getRecommendations(includeInactive)` · `patchRecommendation(id, {dismissed?, addressed?, severity?})` (id URI-encoded; `severity` is what the user saw, so escalation can bring the alert back) |
| DIS | `getDiseaseRisk` |
| TIME | `getTimingWindows` |
| SOIL | `getSoilTests` · `createSoilTest` · `updateSoilTest` · `deleteSoilTest` · `getSoilTestRecommendations` |
| PLANT | `listPlants` · `getPlant` *(defined, not called)* · `createPlant` · `updatePlant` · `deletePlant` · `refreshPlantPlan` |
| INV | `listProducts({category?, includeArchived?})` · `createProduct` · `updateProduct` · `deleteProduct` · `refreshProductProfile` · `linkProductApplications` · `listLoggedProductSuggestions` → `/inventory/suggestions` · `getShoppingList` · `markProductInStock(p)` (a PUT that re-sends the product's facts with `stock_status: "InStock"`) |

`GET /products/:id` has no client function. `/mcp` is not called by the UI.

## Shared Type Mirror

`frontend/src/types/index.ts` hand-mirrors the Rust models that more than one page uses: profile, application, environmental summary and forecast, recommendation, dashboard, calendar, GDD, historical, nitrogen budget, soil-temp forecast, seasonal plan, soil tests and plants. Feature-specific mirrors live beside it and belong to their features: `types/disease.ts` (DIS), `types/timing.ts` (TIME), and `types/inventory.ts`, `inventoryTargets.ts` and `inventoryCompat.ts` (INV). The shared module imports `InventoryStatus`/`ProductNeed` from `types/inventory.ts` for `Recommendation`.

Conventions the mirror follows:

| Rust | TypeScript |
|---|---|
| Unit enum (serde variant name) | String-literal union of the variant names (`"TallFescue"`, `"PreEmergent"`) |
| `Option<T>` | `T \| null` |
| Field the server may omit (`skip_serializing_if`) | Optional `field?: T \| null` (`frac_classes`, `product_id`, `soil_observed_at`, `inventory`). Some always-sent fields are also typed optional (`needs`, `hours_covered`), which is looser than the server. |
| Row id before insert | `id: number \| null` |
| Dates | ISO strings: `YYYY-MM-DD` for calendar dates, RFC 3339 for timestamps |

Display metadata lives next to the type as exhaustive `Record<Enum, …>` maps (`APPLICATION_TYPE_LABELS`, `GRASS_TYPE_LABELS`, `SEVERITY_COLORS`, `SEVERITY_SYMBOLS`, …). Once a new variant is added to the union, every exhaustive map fails to compile until it is labelled. A Rust variant missing from the union goes undetected. Nothing checks the mirror against the server, so a renamed Rust field is a silent `undefined` at runtime. `categoryLabel()` splits a PascalCase category into words ("DiseasePressure" → "Disease Pressure").

## Shared Styles and Utilities

- `styles/shared.ts` has `sharedStyles` (loading, error banner, card, table/th/td, badge, section title, empty state, page title, header row, gauge grid) and `appTypeBadgeStyle()`, a memoized per-`ApplicationType` tinted badge. Styling is inline `React.CSSProperties` throughout. There are no CSS modules and no CSS framework.
- `utils/dates.ts` holds every calendar-date helper. `todayLocalISO()` and `addDaysISO()` build `YYYY-MM-DD` from the **browser's local calendar**, not `toISOString()`. A UTC date is already tomorrow from 8 pm Eastern, and an application logged that evening would be stored a day late and treated as future-dated by the disease and timing models. Used to pre-fill the date fields of the Applications and Soil Tests forms. The authority for "today" is the server's local date (`TZ`, the lawn's timezone, per the HLD); the browser date is only a pre-fill that matches it while the viewer's clock is in the lawn's timezone. When the date in the Applications or Soil Tests form is later than the server's today (learned from the API, not the browser's clock), the form warns that the date is in the future; it does not block the save.
- `utils/units.ts`: `mmToInches` and `formatInches(mm, decimals=2)`. The API carries precipitation in millimetres (lake-native); the UI shows inches.
- The calendar-date display helpers, which format an API `YYYY-MM-DD` date (or its day of month) without a timezone shift by splitting the string, live once in `utils/dates.ts` (`formatShortDate`, `formatMonthDay`, `dayNumber` and the Calendar and Seasonal Plan formatters); pages and feature type modules import them.
- `components/Gauge.tsx` and `gaugeConfigs.ts` are a horizontal bar gauge and its four configs (soil temp 10 cm 75/85 °F, ambient 85/95 °F, humidity 80/90 %, soil moisture 40/45 %). The configs are kept in a separate module so `Gauge.tsx` exports only a component.

## Cross-Cutting Presentation Conventions

### Status palette and "never colour alone"

Status-bearing enums share one palette. Every rendering pairs the colour with a **symbol and a text label**, and the badge text is ink (`#2d3748`) on a tinted background (`<color>26`) with a solid border in the status colour (`TierBadge`, `StateBadge`, `StockBadge`, `InventoryBadge`). The symbol is `aria-hidden` because the label already says it.

| Role | Hex | Used by |
|---|---|---|
| Good | `#0ca30c` | tier Low, window Open/Ideal, In stock, On hand, factor Unfavorable |
| Warning | `#fab219` | tier Moderate, window Closing, stock Low, inventory Low/Partial, factor Marginal |
| Serious | `#ec835a` | tier High |
| Critical | `#d03b3b` | tier Severe, stock Out, Need to buy, factor Favorable |
| Informational | `#3182ce` | window OpeningSoon, Done |
| Neutral | `#a0aec0`, `#718096` | window NotYet, Closed, Blocked |

The maps live with each feature's types (`RISK_TIER_*` and `FACTOR_STATUS_*` in `types/disease.ts`, `WINDOW_STATE_*` in `types/timing.ts`, `STOCK_STATUS_*` and `INVENTORY_STATE_*` in `types/inventory.ts`). This component owns the rule and the palette values; features own the mappings.

Recommendation **severity** uses its own scale in `types/index.ts`: Info `#9ca3af ℹ`, Advisory `#3b82f6 →`, Warning `#eab308 ⚠`, Critical `#ef4444 !` (`AlertCard`, the Recommendations list, `ShoppingListPanel`). The feed owns how it renders: symbol + label in ink on a tint, like the status badges (FEED-REC-004).

### Charts

- **Table beside the chart.** Every chart has a `<details>` "View as table" with the same series: the disease and timing charts (`components/disease/DailyRiskChart.tsx`, `components/timing/SoilSeasonTable.tsx`), the Environmental `TrendChart` and `PredictionChart`, and the dashboard `GddWidget` chart. The owning feature builds the table; this component owns the rule.
- **Forecast vs observed.** Forecast bars are drawn at 45 % fill opacity and today's bar has a 2 px ink outline. The legend explains both ("Outlined = today") (`DailyRiskChart.tsx`).
- **CVD-validated categorical pair.** Comparisons between series use `#2a78d6` (this year; forecast is the same hue, dashed) and `#eb6834` (last year). A reference series is neutral grey `#718096` and also differs by dash pattern, so it is not identified by hue alone (`components/timing/SoilSeasonChart.tsx`).
- **Charting library.** All charts use Recharts (`TrendChart`, `PredictionChart`, `GddWidget`, `DailyRiskChart`, `SoilSeasonChart`).

## Human-readable errors

Every error the UI shows is plain language that says what went wrong and, where there is one, what to do (HLD goal). This holds on every page, widget and form.

- The API client unwraps the backend's `{"error": "<message>"}` body and keeps the status for logic (a 409 is a duplicate). It never shows raw JSON, a bare status code, a stack trace, a file path or an upstream service's response body.
- Known statuses read as explanations: a missing lawn profile (404 on profile-scoped endpoints) points to Settings; a data source that is unavailable or not configured (503) names the source in plain words and says which features are affected; a timeout says the request took too long and to try again; a duplicate (409) uses the server's message.
- The backend writes `error` messages for the owner. Internal and upstream detail (OpenRouter bodies, lake errors, SQL) goes to the log, not the response.
- The `ErrorBoundary` keeps its "Something went wrong" message and the Return to Dashboard action, without the exception text.

## Build, Lint and Type-Check Tooling

| Tool | Configuration | Notes |
|---|---|---|
| Vite 8 (rolldown) | `frontend/vite.config.ts` | Plugins: `esToolkitCompatEsm()` then `@vitejs/plugin-react`. Dev server on 5173, `/api` proxied to `localhost:3000`. |
| TypeScript 7 | `tsconfig.json` (solution: `files: []` + references to `tsconfig.app.json` and `tsconfig.node.json`) | App: ES2023, `strict`, `noUnusedLocals/Parameters`, `erasableSyntaxOnly`, `verbatimModuleSyntax`, bundler resolution, `noEmit`. |
| Biome 2 | `frontend/biome.json` | Recommended rules + React domain, `style/useComponentExportOnlyModules: error`. Ignores `dist` and `public`. `npm run lint` = `biome check .` (lint + format). |
| Build | `npm run build` = `tsc -b && vite build` | Output `frontend/dist`, copied into the image as `/app/static`. |
| Vitest | `npm test` | Shares the Vite config and transform. Covers at least the API client's error mapping (status kept, `{"error": …}` unwrapped, timeout message). CI runs it beside lint and type-check. |

**The es-toolkit redirect.** Recharts imports `es-toolkit/compat/<fn>` deep paths, which resolve only to es-toolkit's CommonJS build. Rolldown mis-compiles those circular CJS modules into a self-referential initializer (`var require_get = require_get()`, minified to `var t = t()`), so any page that renders a chart crashes in the **production** bundle with "t is not a function". Development mode does not show the bug. `esToolkitCompatEsm()` runs with `enforce: "pre"` so it resolves before Vite's own resolver. It maps each `es-toolkit/compat/<fn>` to a virtual module `export { <fn> as default } from 'es-toolkit/compat'`, i.e. the pure-ESM barrel, and leaves the bare barrel import alone. The guard covers every current and future chart page. A suspected bundler bug of this kind is diagnosed by running the production build in a real browser.

**The type-check gotcha.** Because the root `tsconfig.json` is a solution file with no files of its own, `npx tsc --noEmit` checks **zero files** and always passes. The real check is `npx tsc -b` (also the first half of `npm run build`). The CI "Type check" step (`.github/workflows/ci.yml`) runs `npx tsc --noEmit`. The actual type gate on a PR is the Docker build in `docker-publish.yml`, which runs `npm run build`.

**Fast-refresh rule.** `useComponentExportOnlyModules` is why `ErrorBoundary` and `NotFound` are unexported locals of `App.tsx`, and why gauge configs, label maps and helpers live in `.ts` modules apart from the components that use them.

Unit tests (Vitest) guard the logic that pages share, starting with the API client's error mapping, where a regression would put raw JSON or status codes in front of the owner on every page. Rendering and layout rest on the type check, Biome and manual checks of the production build.

## SPA Serving and CORS (backend side)

`backend/src/main.rs` builds the API router, applies a 1 MB request-body limit and the CORS layer, merges the MCP router **after** CORS (when `MCP_TOKEN` is set), and finally attaches the SPA as the router's `fallback_service`:

- `STATIC_DIR` (default `./static`; the image sets `/app/static`). If `STATIC_DIR/index.html` exists, the fallback is `ServeDir(static)` with `index.html` as a plain fallback service, which keeps its 200 status. Otherwise the server logs a warning, serves the API, and answers `GET /` with a plain page saying the frontend was not bundled into this build, so a viewer sees why there is no UI instead of an empty 404.
- A request for an existing asset gets the file. Any other GET or HEAD path outside `/api/v1/`, such as the client route `/timing`, gets `index.html` with status 200: it is a real page, the router takes over, and deep links and reloads work.
- An unknown path under `/api/v1/` never gets the SPA. The API router has its own fallback that answers 404 with the API's JSON error body `{"error": …}`, whether or not the SPA is bundled, so an API caller sees the same error contract as for any other failure.
- Non-GET/HEAD requests to an unmatched path outside `/api/v1/` get 405 from `ServeDir`.

CORS (`build_cors_layer`), from `CORS_ALLOWED_ORIGIN` (`backend/src/config.rs`; blank or whitespace = unset):

| Setting | Layer |
|---|---|
| unset | Allow origin `http://localhost:<SERVER_PORT>` only; methods GET/POST/PUT/PATCH/DELETE; any header. |
| `*` | `CorsLayer::permissive()` with a startup warning. |
| an origin | That origin only, same methods, any header. |
| anything else (not a valid origin) | Startup refused with a configuration error naming `CORS_ALLOWED_ORIGIN`, as for other invalid settings (a short `MCP_TOKEN`, for example). Validated at config load, so the CORS layer never panics. |

The bundled SPA is same-origin, so CORS never applies to it. The Vite dev server proxies, so it is same-origin from the browser too. The variable matters only when the SPA is hosted on another origin. `/mcp` is server-to-server and deliberately outside the CORS layer.

## Failure and Degradation

| Failure | UI behaviour |
|---|---|
| Page chunk fails to load | Render error, caught by `ErrorBoundary` → "Something went wrong" + Return to Dashboard. |
| API returns non-2xx | Page-level inline error in plain language (see Human-readable errors). Other pages are unaffected. |
| Request exceeds its timeout | Same, with a plain message that the request took too long and to try again. |
| One dashboard feature endpoint fails | Only that widget is affected: hidden if it never loaded, its last good data kept (marked stale) if it had (FEED-UI-013). The rest of the dashboard renders. |
| `/dashboard` fails after a successful load | Last good data stays on screen; the error is shown alongside it. |
| Static dir missing | API served; `/` returns a plain page saying the frontend was not bundled. |
| Unknown `/api/v1/...` path | 404 with the JSON error body, never `index.html`. |

## Decisions & Alternatives

| Decision | Chosen | Alternatives Considered | Rationale |
|---|---|---|---|
| Delivery | SPA built by Vite, served as static files by the Axum binary with an `index.html` fallback | Separate web server / Node SSR; Next.js | One container, one origin, no CORS in production, and client routing works on reload. [inferred] |
| Fallback status | `index.html` with 200 for paths outside `/api/v1/`; a JSON 404 for unknown `/api/v1/` paths | `index.html` with 404 for every unmatched path | A deep link is a real page and should not report 404 to tools, caches or the browser. An unknown API path is a genuine miss, and an API caller expects the JSON error body, not HTML. |
| Missing frontend bundle | Plain page at `/` saying the frontend was not bundled | Empty 404 (API-only) | The one viewer who hits it is the owner after a bad build; an empty 404 gives no hint where to look. |
| Invalid `CORS_ALLOWED_ORIGIN` | Refuse to start with a configuration error | Panic; fall back to the localhost default | Falling back would silently break the cross-origin host the setting exists for. Refusing the way other invalid settings are refused surfaces the mistake at deploy with a readable message, not a stack trace. |
| Server state | Per-page fetch on mount via one `fetchJson` wrapper | React Query/SWR; a global store | Small app, one user. Fetch-on-view matches the backend's demand-driven refresh and keeps idle cost at zero. [inferred] |
| Code splitting | `React.lazy` per page, one `Suspense` | Single bundle | Keeps Recharts out of pages without charts and the first load small. [inferred] |
| Dashboard resilience | Feature calls caught to `null` individually | `Promise.all` failing the page | Losing one data source degrades only the features that depend on it (HLD goal). [inferred] |
| Dashboard refresh | 30 s poll while the tab is visible, stopped when hidden | Poll always; no poll | Live-ish conditions while someone is looking; no calls while nobody is (HLD idle-cost goal). [inferred] |
| Error text | Plain language: the server's `{error}` message when it is written for the owner, otherwise a message per status that says what went wrong and what to do | Status + raw body; one generic message for everything | The owner must understand what happened and what to do next (HLD goal). A raw body or status code explains nothing, and a single generic message hides the fix ("set up your lawn profile"). |
| Type sharing | Hand-written TS mirror of serde output | Codegen (ts-rs, OpenAPI) | No build-time coupling between crates and the SPA. [inferred] |
| Status colours | Shared status palette, always symbol + label, ink text on tint | Colour-only chips; per-feature palettes | Colour-blind and greyscale readers get the same information; one palette means "warning" looks the same on every page (stated in `docs/timing-windows.md` and the type modules). |
| Chart accessibility | "View as table" beside every chart | Tooltips only; tables for the disease and timing charts only | Readers who cannot use the plot get the same numbers, on every page. |
| Client timeout vs caller signal | Timeout always applied; caller signal combined with it; caller headers merged | Caller `init` replaces the defaults | A caller (the dashboard cancelling a superseded poll) needs to cancel, but no caller should be able to disable the timeout or drop `Content-Type` by passing an option. |
| Future form date | Warn when the date is after the server's today, still allow the save | Reject; accept silently | A future-dated entry is ignored by the disease and timing models until its day, which the owner should see at entry time. The browser and server calendars can legitimately differ (a device in another timezone), so the owner decides. |
| Narrow screens | Sidebar collapses to a menu at phone width | Fixed 220 px sidebar | The owner checks the lawn from a phone; a fixed sidebar leaves the page too narrow to read. |
| Frontend tests | Vitest | Jest; no runner | Vitest reuses the Vite config and TypeScript transform, so there is no second toolchain to keep in step. |
| Series colours | CVD-validated blue/orange pair, reference series neutral + dashed | Default library palette | Series stay distinguishable under colour-vision deficiency. |
| Bundler | Vite 8 (rolldown) plus the `es-toolkit/compat` ESM redirect | Vite 7 | Vite 7 was blocked by an esbuild CVE flagged in supply-chain scanning. The redirect fixes rolldown's CJS mis-compile of Recharts' dependency and guards every future chart page. |
| Type-check command | `tsc -b` | `tsc --noEmit` | The solution tsconfig makes `--noEmit` a no-op. |
| Lint/format | Biome (lint + format, React domain) | ESLint + Prettier | One fast tool and one config. [inferred] |
| Styling | Inline `CSSProperties` objects + `sharedStyles` | CSS modules; Tailwind | No extra build step; styles live beside the component. [inferred] |
| Browser dates | Local-calendar `YYYY-MM-DD` helpers to pre-fill form dates | `toISOString().slice(0,10)`; asking the server for its date | Every agronomic date is a lawn-local calendar day on the server's clock (`TZ`). The viewer's local calendar matches it for an owner at the lawn; UTC rolls over at 8 pm Eastern and would date evening applications a day late (stated in `utils/dates.ts`). |
| CORS default | Allow only `http://localhost:<port>` when unset | Permissive; none | Same-origin SPA needs nothing; a cross-origin host must be named explicitly. [inferred] |

## Open Questions & Future Decisions

### Resolved

1. ✅ Chart crash in production builds: fixed by the `enforce: "pre"` es-toolkit redirect, which applies to every chart page.
2. ✅ Status is never shown by colour alone: symbol + label on every status badge.
3. ✅ Form dates use the browser's local calendar, not UTC.
4. ✅ Amount fields in the shared mirror are agronomic amounts, displayed as designed: `PhRecommendation.rate_lbs_per_1000sqft` (lime or sulfur), the `NpkRecommendation` nutrient fields (lb per 1,000 sq ft) and `ActivityDetails.rate` (lb N or seed per 1,000 sq ft; pesticide activities carry the placeholder "Label rate", not a number). `NpkRecommendation.product_rate_lbs_per_1000sqft`, a nutrient amount converted into pounds of an example-ratio product, is also designed: converting a nutrient amount into a fertilizer amount is agronomy (soil-tests owns it; the mirror only carries it).

### Deferred

1. **CI type check is a no-op.** `ci.yml` runs `npx tsc --noEmit`, which checks no files. Type errors are caught only by the Docker build in `docker-publish.yml`. The CI step should be `npx tsc -b`.
2. **Divergence: severity badges are white on a solid fill** (FEED-REC-004, owned by recommendation-feed). `SEVERITY_COLORS` renders white text on a solid fill; white on `#eab308` (Warning) has poor contrast. Intent: symbol + label in ink on a tint, like the status badges.
3. **Colour-only indicators remain.** **Divergence:** `Gauge` signals warn/critical by bar colour alone, with no symbol or label (`components/Gauge.tsx:23-27`); intent is a text label beside the bar (WX-VIEW-PAGE-011, owned by weather-data). `CRABGRASS_STATUS_COLORS`, `ACTIVITY_STATUS_COLORS`, `NUTRIENT_LEVEL_COLORS`, `PREDICTION_CONFIDENCE_COLORS` and `IDENTIFICATION_CONFIDENCE_COLORS` use an older palette; whether each is always paired with a label is up to the owning feature.
4. **Divergence: table views are not universal** (UI-CONV-003). `TrendChart`, `PredictionChart` (Environmental) and the `GddWidget` chart have no table view. Intent: every chart has "View as table".
5. **Gauge thresholds are one-directional.** The soil-moisture gauge flags only high values (40/45 %, `components/gaugeConfigs.ts:33-39`). The <0.10 "irrigation needed" threshold is not shown.
6. **Divergence: error bodies are shown raw** (UI-ERR-004, UI-ERR-005). The client does not unwrap `{"error": …}` (`api/client.ts:53-61`; body built at `backend/src/error.rs`), so users see JSON, e.g. `409: {"error":"An identical entry…"}`, and pages render it as is (for example `pages/DiseaseRisk.tsx:21-32`, the Timing page). Intent: Human-readable errors.
7. **Divergence: `fetchJson` header/signal override** (UI-API-005, UI-API-006, UI-DASH-003). Spreading `init` after the defaults (`api/client.ts:48-52`) lets a caller's `headers` drop `Content-Type` and its `signal` disable the timeout. The Dashboard's own `AbortController` is never passed to `fetch` (`pages/Dashboard.tsx:61-64`): aborting only discards results, and superseded requests keep running. Intent: headers merged, timeout always applied, superseded polls aborted.
8. **Divergence: an empty 2xx body throws** (UI-API-003). A 2xx with an empty non-204 body throws a JSON parse error (`api/client.ts:64-65`). Intent: treated as no content. Separately and still open, `res.json()` is cast to `T` without runtime validation.
9. **Divergence: deep links return 404** (UI-SRV-001, UI-SRV-008). The SPA fallback sends `index.html` with status 404, and unknown `/api/v1/...` paths return HTML instead of JSON (`backend/src/main.rs:203-205`). Intent: client routes get `index.html` with 200; unknown API paths get a JSON 404.
10. **Unused exports and assets.** `getHealth` and `getPlant` are never called. `GET /products/:id` has no client. `src/assets/hero.png` and `vite.svg` are unreferenced. **Divergence:** the `index.html` `<title>` is "frontend" (UI-SHELL-007). Intent: "TurfOps".
11. **Divergence: browser date vs. server date** (UI-UTIL-005, UI-UTIL-006). `todayLocalISO()` (`utils/dates.ts:12-14`) pre-fills form dates from the viewer's clock; the backend's "today" is the server's local date (`TZ`). They disagree when the viewer's device is in another timezone (the owner travelling, or a device set to another zone) around midnight: a date a day ahead is read as future-dated by the disease and timing models until the server's day catches up, and a day behind backdates the entry. The form does not warn and the API does not reject either. Separately, date-only display helpers that parse at local midnight or noon are duplicated per feature (`formatShortDate`, `formatMonthDay`, `pages/SeasonalPlan.tsx:29`, `pages/Calendar.tsx:18`) instead of living in `utils/dates.ts`. Intent: the form warns on a date after the server's today, and the display helpers live once in `utils/dates.ts`.
12. **Files over the 300-line limit** owned here: `types/index.ts` (694), `api/client.ts` (403), `pages/Dashboard.tsx` (372). (`pages/Settings.tsx`, 304 lines, is applications' debt.) The `Dashboard` component function exceeds 50 lines. Split `client.ts` and `types/index.ts` by owning component.
13. **Divergence: no responsive layout** (UI-NAV-003). The 220 px sidebar never collapses (`components/Layout.tsx`). Intent: it collapses to a menu at phone width.
14. **Divergence: no frontend tests** (UI-BUILD-008, UI-BUILD-009). No test runner is configured and CI runs none. Intent: Vitest, covering at least the API client's error mapping, run in CI.
15. **Divergence: a failed re-poll blanks a widget** (FEED-UI-013, owned by recommendation-feed). Each feature call on the dashboard is caught to `null` on every poll (`pages/Dashboard.tsx:66-80`), so one failed 30 s re-poll hides a widget that was showing good data. Intent: the widget keeps its last good data, marked stale.
16. **Divergence: invalid `CORS_ALLOWED_ORIGIN` panics at startup** (UI-SRV-009). `backend/src/main.rs:250` calls `expect("Invalid CORS_ALLOWED_ORIGIN")`. Intent: config load refuses the value with a configuration error naming the variable. A blank or whitespace value counts as unset (`backend/src/config.rs:260-262`).
17. **Divergence: missing static directory is silent to the browser** (UI-SRV-010). Without `STATIC_DIR/index.html` the server logs one warning and runs API-only (`backend/src/main.rs:198-208`); `/` returns an empty 404. Intent: `/` serves a plain page saying the frontend was not bundled.

## References

- `frontend/src/main.tsx`, `frontend/src/App.tsx`, `frontend/src/index.css`, `frontend/index.html`
- `frontend/src/components/Layout.tsx`, `pages/Dashboard.tsx`
- `frontend/src/api/client.ts`
- `frontend/src/types/index.ts` (and the feature mirrors `types/disease.ts`, `types/timing.ts`, `types/inventory.ts`)
- `frontend/src/styles/shared.ts`, `frontend/src/utils/dates.ts`, `frontend/src/utils/units.ts`
- `frontend/vite.config.ts`, `frontend/biome.json`, `frontend/tsconfig*.json`, `frontend/package.json`, `frontend/README.md`
- `components/disease/TierBadge.tsx`, `components/disease/DailyRiskChart.tsx`, `components/timing/SoilSeasonChart.tsx`, `components/timing/SoilSeasonTable.tsx`, `components/InventoryBadge.tsx`, `components/AlertCard.tsx`
- `backend/src/main.rs` (router, SPA fallback, `build_cors_layer`), `backend/src/config.rs` (`CORS_ALLOWED_ORIGIN`, `SERVER_PORT`), `backend/src/error.rs` (error body)
- `.github/workflows/ci.yml`, `.github/workflows/docker-publish.yml`, `Dockerfile` (frontend build stage, `STATIC_DIR`)
- `docs/high-level-design.md`; `docs/timing-windows.md` (card and badge conventions)
