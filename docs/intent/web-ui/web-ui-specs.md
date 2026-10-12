# Web UI Shell & Conventions — EARS Specs

Design: [web-ui-design.md](web-ui-design.md). Prefix `UI`. Facets: `SHELL` (entry, routes, error boundary, not-found page, page loading), `NAV` (sidebar), `DASH` (dashboard data loading), `API` (the typed API client), `ERR` (human-readable errors, client and API error body), `TYPE` (shared TypeScript mirror), `UTIL` (shared date, unit and label helpers), `CONV` (cross-cutting presentation conventions), `BUILD` (build, lint and type-check tooling), `SRV` (SPA serving, request limits and CORS).

"Status palette" below means the shared roles Good `#0ca30c`, Warning `#fab219`, Serious `#ec835a`, Critical `#d03b3b`, Informational `#3182ce` and Neutral `#a0aec0` / `#718096`. Feature pages, their widgets and their own badges are specified by their owners (FEED-UI-*, DIS-UI-*, TIME-UI-*, WX-VIEW-PAGE-*, WX-OUTLOOK-UI-*, RULE-UI-*); the dashboard's layout, polling and per-widget hiding are FEED-UI-009 to FEED-UI-011.

## Shell and routing

- [x] **UI-SHELL-001**: The web UI shall render every route inside one layout route, so the sidebar persists across navigation, with these routes: `/` (Dashboard), `/applications`, `/inventory`, `/landscape`, `/calendar`, `/environmental`, `/recommendations`, `/disease-risk`, `/disease-risk/:slug`, `/timing`, `/soil-tests`, `/seasonal-plan`, `/settings`, and a `*` wildcard.
- [x] **UI-SHELL-002**: When the browser is on a path no route matches, the web UI shall show "Page Not Found" inside the layout with a "Go to Dashboard" link to `/`.
- [x] **UI-SHELL-003**: The web UI shall load each page as its own lazily imported chunk and show "Loading..." while a page chunk loads.
- [x] **UI-SHELL-004**: If rendering any page throws (a failed page-chunk load included), then the web UI's error boundary shall replace the app with "Something went wrong" and a "Return to Dashboard" button that clears the error and does a full navigation to `/`.
- [x] **UI-SHELL-005**: Each web UI page shall catch its own data-fetch failures and show them inline on that page, so a failed API request never reaches the error boundary or affects another page.
- [x] **UI-SHELL-006**: Each web UI page shall fetch the data it shows when it mounts, with no global client-side store or server-state cache, and no page other than the dashboard and the Environmental page shall re-fetch on a timer.
- [x] **UI-SHELL-007**: The web UI's HTML document (`frontend/index.html`) shall have the `<title>` "TurfOps".

## Navigation

- [x] **UI-NAV-001**: The web UI sidebar shall be a navigation landmark labelled "Main navigation" with the "TurfOps" wordmark and twelve links in this order: Dashboard, Applications, Inventory, Landscape, Calendar, Environmental, Recommendations, Disease Risk, Seed & Pre-Em Timing, Soil Tests, Seasonal Plan, Settings.
- [x] **UI-NAV-002**: The web UI sidebar shall mark the active link by bold weight as well as color and background, and shall mark the Dashboard link active only on `/` itself.
- [x] **UI-NAV-003**: While the browser viewport is too narrow for the 220 px web UI sidebar beside the page (phone width), the web UI shall collapse the sidebar into a narrow-screen layout that gives the page the full width and keeps all twelve nav links reachable from a menu control inside the "Main navigation" landmark.

## Dashboard data loading

- [x] **UI-DASH-001**: When the dashboard loads, it shall request `/api/v1/dashboard` together with the GDD, nitrogen budget, soil-temperature outlook, disease risk and timing-window endpoints in parallel, and only a failed `/api/v1/dashboard` request shall be a page-level error.
- [x] **UI-DASH-002**: If the dashboard's `/api/v1/dashboard` request fails after an earlier load succeeded, then the dashboard shall keep showing the last good data with the error in a banner beside it.
- [x] **UI-DASH-003**: When a new dashboard poll starts while the previous dashboard poll's requests are still in flight, the dashboard shall abort those requests through the API client (passing its abort signal to `fetch`), not only discard their results.

## API client

- [x] **UI-API-001**: The web UI shall make every API request through the one typed client module (`api/client.ts`), which calls the relative base `/api/v1` and returns a typed promise per endpoint; no page or component shall call `fetch` itself.
- [x] **UI-API-002**: The API client shall abort a request that exceeds its timeout: 30 seconds for the seasonal plan and the shopping list, 60 seconds for creating a plant, refreshing a plant's care plan, creating a product and refreshing a product profile, and 15 seconds for every other request.
- [x] **UI-API-003**: When an API response is 204 or any other 2xx response with an empty body, the API client shall resolve with no value instead of raising a JSON parse error; for any other 2xx response it shall resolve with the parsed JSON body.
- [x] **UI-API-004**: The API client shall URI-encode values taken from data or user text before putting them in a path or query string (the recommendation id in `PATCH /recommendations/:id`, the product name in `/frac-classes?product=`).
- [x] **UI-API-005**: When a caller passes its own headers to an API client request, the API client shall merge them with its default headers (`Content-Type: application/json`) instead of replacing the defaults.
- [x] **UI-API-006**: The API client shall apply its per-request timeout (UI-API-002) to every request even when the caller passes its own abort signal, so either the caller's signal or the timeout aborts the request and a caller can never remove the timeout.

## Human-readable errors

- [x] **UI-ERR-001**: When an `/api/v1` handler fails, the API shall respond with the JSON body `{"error": "<message>"}` and status 404 (not found), 400 (invalid data), 409 (duplicate application-log entry), 503 (data source unavailable) or 500 (anything else).
- [x] **UI-ERR-002**: If an `/api/v1` request fails for an internal reason (configuration, database, I/O or parse error), then the API shall log the detail and respond 500 with the error message "Something went wrong on the server. Try again; the details are in the server log."; a weather-lake query failure is instead the lake's 503 (WX-SRC-LAKE-014).
- [x] **UI-ERR-003**: The API's `error` message shall be written for the owner and shall not contain upstream response bodies, raw LLM output, URLs or internal error text (OpenRouter, OpenWeatherMap, Home Assistant or lake details); that detail shall go to the log only.
- [x] **UI-ERR-004**: When an API request returns a non-2xx status, the API client shall raise an error that carries the HTTP status (for page logic) and the `error` message unwrapped from the `{"error": …}` body, never the raw body text.
- [x] **UI-ERR-005**: Every error message the web UI shows (pages, widgets and forms) shall be plain language saying what went wrong and, where there is one, what to do, and shall never show raw JSON, a bare HTTP status code, a stack trace, a file path or an upstream service's response body.
- [x] **UI-ERR-006**: When a profile-scoped API request returns 404 because no lawn profile exists, the web UI shall say the lawn profile is not set up and point the owner to Settings.
- [x] **UI-ERR-007**: When an API request returns 503, the web UI shall name the unavailable or unconfigured data source in plain words and say which features are affected.
- [x] **UI-ERR-008**: When an API request exceeds its timeout, the web UI shall say the request took too long and to try again, without the timeout in milliseconds.
- [x] **UI-ERR-009**: When an API request returns 400 or 409, the web UI shall show the server's `error` message as written, with no status prefix.
- [x] **UI-ERR-010**: When an API request returns 500, the web UI shall say that something went wrong on the server and to try again, without the response body.
- [x] **UI-ERR-011**: The web UI's error boundary shall not show the exception's message text, only "Something went wrong" and the Return to Dashboard action.

## Shared type mirror

- [x] **UI-TYPE-001**: The web UI shall mirror the Rust models shared by more than one page in `types/index.ts`, and feature-specific models in that feature's own module (`types/disease.ts`, `types/timing.ts`, `types/inventory.ts`).
- [x] **UI-TYPE-002**: The web UI type mirror shall represent each serde unit enum as a string-literal union of its Rust variant names, each `Option<T>` as `T | null`, each field the server may omit as optional, and dates as ISO strings (`YYYY-MM-DD` for calendar dates, RFC 3339 for timestamps).
- [x] **UI-TYPE-003**: The web UI shall keep display labels, colors and symbols for an enum in an exhaustive `Record<Enum, …>` map beside the type, so adding a variant to the union fails type-checking until every map labels it.
- [x] **UI-TYPE-004**: The web UI shall compute no agronomic result (dates, tiers, window states, amounts, recommendations); it shall only format, label and color what the API returns.

## Shared helpers

- [x] **UI-UTIL-001**: When a web UI form pre-fills a date field with today (application and soil-test forms), it shall use the browser's local calendar date, never the UTC date from `toISOString()`.
- [x] **UI-UTIL-002**: Wherever the web UI displays a calendar date (`YYYY-MM-DD`) from the API, it shall show that same calendar day, parsing it on the local calendar or splitting the string, never as UTC midnight.
- [x] **UI-UTIL-003**: Wherever the web UI shows precipitation, it shall convert the API's millimetres to inches in the browser (25.4 mm per inch), formatted to two decimals with "in" by default.
- [x] **UI-UTIL-004**: When the web UI shows a recommendation category, it shall split the PascalCase variant into words ("DiseasePressure" → "Disease Pressure").
- [x] **UI-UTIL-005**: When the date in the web UI's application or soil-test form is later than the server's today (the lawn-local date under `TZ`, not the browser's clock), the form shall warn that the date is in the future, without blocking the save.
- [x] **UI-UTIL-006**: The web UI shall define its calendar-date display helpers (formatting an API `YYYY-MM-DD` date, or its day of month, without a timezone shift) once in `utils/dates.ts`, and pages and feature type modules shall import them rather than define their own.

## Presentation conventions

- [x] **UI-CONV-001**: Wherever the web UI shows a status-bearing value as a badge (disease tier, contributing factor, timing-window state, stock status, inventory status), it shall show the status symbol and a text label in ink `#2d3748` on a tint of the status color with a border in that color, never the color alone, with the symbol hidden from assistive technology.
- [x] **UI-CONV-002**: The web UI shall color status badges only from the status palette: Good for Low tier, Open/Ideal, In stock, On hand and Unfavorable; Warning for Moderate, Closing, stock Low, inventory Low/Partial and Marginal; Serious for High; Critical for Severe, Out, Need to buy and Favorable; Informational for OpeningSoon and Done; Neutral for NotYet, Closed and Blocked.
- [x] **UI-CONV-003**: Where any web UI chart plots a series (the disease and timing charts, the Environmental trend and prediction charts and the dashboard GDD widget's chart alike), the chart shall offer a "View as table" disclosure listing the same values.
- [x] **UI-CONV-004**: Where a web UI chart shows forecast values beside observed ones, it shall draw the forecast distinctly (faded fill for bars, dashed for lines), and where it marks today it shall explain the marking in its legend.
- [x] **UI-CONV-005**: Where a web UI chart distinguishes two compared series by hue, it shall use `#2a78d6` (this year; its forecast the same hue dashed) and `#eb6834` (last year), and draw any reference series in neutral `#718096` with its own dash pattern so it is not identified by hue alone.

## Build, lint and type-check tooling

- [x] **UI-BUILD-001**: The frontend production build (`npm run build`) shall type-check the whole app with `tsc -b` before bundling with Vite into `frontend/dist`, and fail on any type error.
- [x] **UI-BUILD-002**: The frontend TypeScript configuration shall enable `strict`, `noUnusedLocals`, `noUnusedParameters`, `erasableSyntaxOnly` and `verbatimModuleSyntax`.
- [x] **UI-BUILD-003**: The frontend lint (`npm run lint`) shall run, as configured in `frontend/biome.jsonc`, Biome's recommended rules with the React domain and `useComponentExportOnlyModules` as an error, plus Biome's format check, over everything but `dist` and `public`.
- [x] **UI-BUILD-004**: The frontend bundler shall redirect every `es-toolkit/compat/<fn>` deep import to a named re-export of the `es-toolkit/compat` ESM barrel, resolved before Vite's own resolver, so charts do not crash in the production bundle.
- [x] **UI-BUILD-005**: The Vite dev server shall run on port 5173 and proxy `/api` to `http://localhost:3000`, so the SPA calls the API same-origin in development.
- [x] **UI-BUILD-006**: The CI frontend job shall run `npm run lint` on every push to `main` and every pull request against `main`.
- [x] **UI-BUILD-007**: The CI frontend job shall type-check the app with `npx tsc -b` (the root `tsconfig.json` is a solution file, so `tsc --noEmit` checks no files).
- [x] **UI-BUILD-008**: The frontend shall have a Vitest test runner (`npm test`) whose suite covers at least the API client's error mapping: the HTTP status kept and the `error` message unwrapped from `{"error": …}` (UI-ERR-004), and the plain timeout message (UI-ERR-008).
- [x] **UI-BUILD-009**: The CI frontend job shall run the frontend tests (`npm test`) on every push to `main` and every pull request against `main`.
- [x] **UI-BUILD-010**: Frontend component tests shall render in jsdom with Testing Library, each opting in with a `// @vitest-environment jsdom` docblock while other suites run in Node, and the shared setup (`src/test/setup.ts`) shall register the jest-dom matchers, clean up after every test, and stub `ResizeObserver` and `matchMedia`.

## SPA serving and CORS

- [x] **UI-SRV-001**: Where `STATIC_DIR/index.html` exists (default `./static`), the server shall serve existing files from `STATIC_DIR` and answer every other unmatched GET or HEAD path outside `/api/v1/` with `index.html` and status 200, so client routes load on reload and from deep links.
- [x] **UI-SRV-002**: If `STATIC_DIR/index.html` does not exist, then the server shall log a warning at startup and serve the API without the SPA.
- [x] **UI-SRV-003**: The server shall reject `/api/v1` request bodies larger than 1 MB.
- [x] **UI-SRV-004**: While `CORS_ALLOWED_ORIGIN` is unset, blank or whitespace, the server shall allow cross-origin `/api/v1` requests only from `http://localhost:<SERVER_PORT>`, with methods GET, POST, PUT, PATCH and DELETE and any header.
- [x] **UI-SRV-005**: Where `CORS_ALLOWED_ORIGIN` is `*`, the server shall allow any origin and log a warning at startup.
- [x] **UI-SRV-006**: Where `CORS_ALLOWED_ORIGIN` names one origin, the server shall allow cross-origin `/api/v1` requests only from that origin, with methods GET, POST, PUT, PATCH and DELETE and any header.
- [x] **UI-SRV-008**: When a request path under `/api/v1/` matches no API route, the server shall respond 404 with the JSON API error body `{"error": "<message>"}` (UI-ERR-001), never the SPA's `index.html`, whether or not the SPA is bundled.
- [x] **UI-SRV-009**: If `CORS_ALLOWED_ORIGIN` is set to a value other than `*` that is not a valid origin, then the server shall refuse to start with a configuration error naming `CORS_ALLOWED_ORIGIN`, the same way it refuses other invalid settings, never by a panic.
- [x] **UI-SRV-010**: While `STATIC_DIR/index.html` does not exist, the server shall answer `GET /` with a plain page saying the frontend was not bundled into this build.
