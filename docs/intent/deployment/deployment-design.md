---
parent: high-level-design
prefix: DEPLOY
---

# Deployment, Configuration & Operations

## Context and Design Philosophy

TurfOps ships as one statically served binary in one container image that runs next to a PostgreSQL database. The repo provides the image, its configuration surface and an example Compose file, and each works for anyone who wants to run the project; the owner's own deployment is built separately and is out of scope here. This component owns everything between the source tree and a running, observable process: the image build, the example Compose stack, environment configuration, startup order, router assembly and middleware, the database pool and migration run, the HTTP error contract, the health endpoint, logging, and the CI / supply-chain gates that guard `main`.

Guiding principles:

- **Small, non-root, distroless runtime.** The runtime image carries the binary, the built SPA and the timezone database, and nothing else: no shell, no package manager, no C++ runtime.
- **Configuration is the environment.** Every setting is an environment variable with a safe default. Optional integrations are off when their key is unset or empty; nothing optional can stop the app from starting. Only the database password is required (unset or empty refuses to start). A malformed `MCP_TOKEN` or `CORS_ALLOWED_ORIGIN` refuses to start rather than run weakly, and a `TZ` that names no timezone refuses to start rather than run on the wrong clock. Any other unusable value falls back to its default with a warning naming the setting.
- **Secrets never leave the process.** Config structs holding a credential redact it in `Debug`, error responses never echo internals, and `.env` files are never committed or copied into an image. The OpenWeatherMap key, which must travel in the request URL, is stripped from every error its client builds.
- **`TZ` is the app's one clock.** "Today" and every date gate in every component is the lawn's local date, taken from `TZ` (the app's `Clock`, `backend/src/clock.rs`, and `chrono::Local` where a rule reads the date directly), so the deployment sets `TZ` to the lawn's timezone. A process running on UTC computes wrong dates, not just odd-looking ones. Startup logs the zone it resolved and warns when `TZ` is unset.
- **The lake is mounted, never written.** The NOAA weather lake is a read-only bind mount; the app reads it in place (owned by Weather data access).
- **Graceful stop.** On SIGTERM the server finishes in-flight requests before exiting, so stopping or restarting the container does not cut a request off midway.
- **Gates before merge.** Format, clippy, tests, type check, lint and a supply-chain scan run on every PR; dependency updates arrive continuously through Renovate.

## Container Image (`Dockerfile`)

Three stages, every base image pinned by digest (Renovate maintains the digests).

| Stage | Base | Produces |
|---|---|---|
| `frontend-build` | `node:26-alpine` | `npm ci --ignore-scripts` then `npm run build` → `/app/frontend/dist` |
| `backend-build` | `rust:1.98-slim-trixie` | `cargo build --release` → `/app/target/release/turfops-backend` |
| runtime | `cgr.dev/chainguard/glibc-dynamic:latest@sha256:…` | `/app/turfops-backend`, `/app/static`, `/usr/share/zoneinfo` |

**No install scripts.** The frontend stage installs with `--ignore-scripts`, the same policy as the supply-chain scan, so no package's lifecycle script runs in the image build. No dependency in the lockfile declares an install script; one that genuinely needs it would be named in the Dockerfile with the reason.

**Native toolchain.** The build stage installs `g++` because the `duckdb` crate is built with `bundled` (its C++ amalgamation compiles into the binary) and `tzdata` only so its files can be copied forward.

**Static C++ runtime.** `RUSTFLAGS="-C link-arg=-static-libstdc++ -C link-arg=-static-libgcc"` links libstdc++ and libgcc into the binary. The Chainguard `glibc-dynamic` runtime provides glibc but not `libstdc++.so.6`, which bundled DuckDB otherwise needs at load time. The binary's only dynamic dependency is glibc.

**Dependency caching.** `Cargo.toml` / `Cargo.lock` are copied first and built against a placeholder `main.rs` (`cargo build --release 2>/dev/null || true`), so the dependency layer is reused until the manifests change. The real `src/` is then copied and `touch src/main.rs` forces the crate itself to rebuild.

**MCP resources.** `docs/timing-windows.md` and `docs/agronomy-methods.md` are copied to `/docs/` in the build stage. The MCP server embeds them with `include_str!("../../../docs/…")` from `backend/src/mcp/resources.rs`; from `/app/src/mcp/` that path resolves to `/docs/`, and from the repo checkout it resolves to the repo's `docs/`. Both copies must exist at compile time or the build fails. `.dockerignore`'s `*.md` is root-anchored, so it excludes only top-level Markdown and leaves `docs/*.md` in the context.

**Timezone database and `TZ`.** `TZ` is resolved once, in `Config::from_env`, by `Clock::from_tz` (`backend/src/clock.rs`): an IANA name becomes `Clock::Zone`, looked up in the zone database compiled into the binary by `chrono-tz` (0.10.4), and unset or blank becomes `Clock::System`, the host's default zone. The resolved `Clock` is stored as `config.clock` and handed to `AppState`, data sync and the OpenWeatherMap client. `/usr/share/zoneinfo` is copied from the build stage for `chrono::Local`, which the rules and a few endpoints read directly: without it `Local` silently stays on UTC whatever `TZ` says, and would disagree with the `Clock`. The image itself sets no `TZ`, because it cannot know where the lawn is; whoever runs it supplies the lawn's timezone (the example Compose passes one). `TZ` is the single "today" clock for the whole app (HLD, Key Design Decisions): window days, freeze nights, fungicide residual windows, the nitrogen budget's year, plan and calendar dates, and the local calendar days that observed lake data and forecast slots are bucketed into. Without `TZ` the clock in the image is UTC, and without the zoneinfo files so is `chrono::Local`. In the eastern US every date gate is then a day ahead from evening (20:00 EDT / 19:00 EST) until midnight, and the year rolls over early on New Year's Eve. That is a correctness failure, so running without `TZ` is unsupported, and the backend makes the clock visible instead of trusting it:

- At startup `main.rs` logs at INFO `Clock: TZ=<zone> (UTC<offset>), local time <date> <time>` (`Clock::describe`), so a wrong clock shows in the startup log.
- When `TZ` is unset or empty it logs a warning that every date gate follows the host's default zone (UTC in the container image) and that `TZ` must be set to the lawn's timezone. It still starts: a local `cargo run` legitimately uses the developer's OS zone, while in the image the default is UTC.
- When `TZ` is set but is not a timezone name (a typo, or a name the zone database lacks), `Clock::from_tz` returns a configuration error naming `TZ` and the value, `Config::from_env` fails, and `main` exits non-zero, because chrono would otherwise fall back to UTC without a word.
- `GET /api/v1/today` (`backend/src/api/today.rs`) exposes the clock to the web UI as `{"today": "YYYY-MM-DD", "timezone": <IANA name or null>}`, with no database or data-source access, so forms judge "future" by the lawn's date rather than the viewer's (DEPLOY-TZ-006). It is its own route, not part of `/health`, which stays an operations probe.

**Runtime.** Binary and static files are copied `--chown=65532:65532`, and the image declares `USER 65532:65532` explicitly, so the process stays non-root even if the base image's default user changes. `ENV STATIC_DIR=/app/static`, `EXPOSE 3000`, `ENTRYPOINT ["./turfops-backend"]`.

**Health check.** The runtime has no shell or curl, so the image's `HEALTHCHECK` runs the binary itself: `turfops-backend --healthcheck` sends `GET /api/v1/health` to `127.0.0.1:$SERVER_PORT` with a 3 s timeout and exits 0 only on a 200 (`backend/src/health_probe.rs`). Interval 30 s, timeout 5 s, 3 retries; the 90 s start period covers the startup wait for PostgreSQL plus migrations. Living in the image, it applies to any deployment of it, not only Compose.

**Release profile** (`backend/Cargo.toml`): `strip = true`, `lto = true`, `codegen-units = 1`.

## Example Compose Stack (`docker-compose.yml`)

`docker-compose.yml` is an example for anyone who wants to try the project: it builds the image from source and runs it next to PostgreSQL 18, with the weather lake mounted from the host. It assumes nothing about a particular host beyond the variables below.

| Service | Image / build | Ports | Notes |
|---|---|---|---|
| `db` | `postgres:18-alpine@sha256:…` | `127.0.0.1:5433:5432` | `POSTGRES_DB` / `POSTGRES_USER` = `turfops`; `POSTGRES_PASSWORD: ${DB_PASSWORD:?}`; volume `turfops_data:/var/lib/postgresql`; healthcheck `pg_isready -U turfops` every 5 s, 5 retries |
| `app` | `build: .` (Dockerfile) | `3000:3000` | `depends_on: db: condition: service_healthy`; lake mount `${DATALAKE_HOST_PATH:-/data}:/data:ro` |

Both services `restart: unless-stopped`. Host port 5433 exposes Postgres for a local `cargo run` against the Compose database (`README.md`, Local development); it is bound to the loopback interface, so other machines on the network cannot reach the database.

**Postgres data volume.** The PostgreSQL 18 image keeps its cluster in a version-specific directory (`/var/lib/postgresql/18/docker`) under the volume root `/var/lib/postgresql`. The named volume is mounted at that root, so a fresh `docker compose up` initializes the database and the data survives recreating the container.

**Required password.** `DB_PASSWORD` is read with `${DB_PASSWORD:?}`: Compose refuses to start when it is unset or empty, and the same value feeds both `POSTGRES_PASSWORD` and the app's `DATABASE_PASSWORD`. The bare `:?` form (no message) is deliberate: the ggshield pre-commit hook flags `${VAR:?message}` as a hard-coded password.

**Env passthrough.** Every optional key is passed as `${KEY:-}` or `${KEY:-default}`, so an unset host variable reaches the app as an **empty string**. Values come from the shell environment or from a `.env` file beside the Compose file, which Compose reads on its own and `.gitignore` keeps out of the repo. The app's "empty = not configured" rule exists for exactly this. `SERVER_HOST`, `SERVER_PORT` and `STATIC_DIR` are fixed in the file; `TZ` defaults to `America/New_York` and should be set to the lawn's timezone; `RUST_LOG` defaults to `info`.

**Pre-built image.** CI publishes the image to `ghcr.io/tgrecojr/turfops`; the README shows replacing `build:` with `image:` to run it without building.

## Configuration (`backend/src/config.rs`)

`Config::from_env()` reads everything once at startup. Two variables are read outside it: `STATIC_DIR` (`main.rs`) and `DB_MAX_CONNECTIONS` (`db/pool.rs`). "Compose" is the value the Compose file passes when the host leaves the variable unset.

| Variable | App default | Compose | Empty / invalid value |
|---|---|---|---|
| `DATABASE_HOST` / `_PORT` / `_NAME` / `_USER` | `localhost` / `5432` / `turfops` / `turfops` | `db` / `5432` / `turfops` / `turfops` | bad port → warn, 5432 |
| `DATABASE_PASSWORD` | **required** | `${DB_PASSWORD:?}` | unset or empty → `Config` error naming it, process exits |
| `DB_MAX_CONNECTIONS` | 10 | not passed | unparseable → warn, 10 |
| `SERVER_HOST` / `SERVER_PORT` | `0.0.0.0` / `3000` | fixed `0.0.0.0` / `3000` | bad host → warn, `0.0.0.0`; bad port → warn, 3000 |
| `STATIC_DIR` | `./static` | `/app/static` | missing `index.html` → API-only mode (warn) |
| `CORS_ALLOWED_ORIGIN` | unset → `http://localhost:<port>` | `""` | empty = unset; `*` = permissive (warn); an unparseable origin **panics** at startup |
| `DATALAKE_ROOT` | `/data` | `/data` | paths derive as `<root>/silver/weather/silver_weather.parquet`, `<root>/gold/weather/daily_weather.parquet`; logged at INFO |
| `WEATHER_SILVER_PATH` / `WEATHER_GOLD_PATH` | derived | not passed | override the derived path; not checked at startup (first read reports) |
| `NOAA_STATION_WBANNO` | 3761 | 3761 | unparseable → warn, 3761 |
| `HA_URL` / `HA_TOKEN` | `http://localhost:8123` / `""` | `""` / `""` | Home Assistant is configured iff the token is non-empty (`logic/data_sync.rs`) |
| `HA_TEMPERATURE_ENTITY` / `HA_HUMIDITY_ENTITY` | `sensor.temp_humidity_sensor_temperature` / `…_humidity` | same | — |
| `HA_TEMPERATURE_UNIT` | Fahrenheit | `fahrenheit` | `celsius` → Celsius; `fahrenheit` → Fahrenheit; anything else → warn, Fahrenheit |
| `OWM_API_KEY` | unset | `""` | empty/whitespace = not configured (whole `OpenWeatherMapConfig` is `None`) |
| `OWM_LATITUDE` / `OWM_LONGITUDE` | 0 / 0 | 0 / 0 | unparseable → warn, 0.0 |
| `OWM_ENABLED`, `OPENROUTER_ENABLED` | on | `true` | `true`, `1` or `yes` in any letter case → on; any other value → off |
| `OPENROUTER_API_KEY` | unset | `""` | empty/whitespace = not configured |
| `OPENROUTER_MODEL` | `anthropic/claude-haiku-4.5` | same | not validated |
| `OPENROUTER_BASE_URL` | `https://openrouter.ai/api/v1` | not passed | — |
| `MCP_TOKEN` | unset | `""` | trimmed; empty = `/mcp` not mounted; set but < 32 chars → `Config` error, process exits (`mcp/config.rs`) |
| `LAWN_NAME`, `LAWN_GRASS_TYPE`, `LAWN_USDA_ZONE` | `Main Lawn`, `TallFescue`, `7a` | same | unknown grass → warn, TallFescue |
| `LAWN_SOIL_TYPE`, `LAWN_SIZE_SQFT`, `LAWN_IRRIGATION_TYPE` | none | `Loam`, `5000`, `InGround` | unknown / unparseable → warn, none |
| `TZ` | process default (UTC in the image, the OS zone under `cargo run`) | `America/New_York` | **required in practice**: the lawn's timezone, the app's single "today" clock; unset or empty → warn, process default; set but not a timezone → `Config` error naming `TZ`, process exits |
| `RUST_LOG` | `info` | `info` | invalid filter → warn, `info` |

Every fallback caused by an unusable non-empty value logs a warning naming the setting and the default used, so a typo shows in the startup log instead of quietly changing behavior. An empty value is "not set", not invalid, and takes the default without a warning (except `TZ`, whose absence is always worth a warning).

`LAWN_*` values only seed the lawn profile on first start (`ensure_default_profile`); once a profile exists they are ignored and the profile is edited through Settings.

**Secret redaction.** `HomeAssistantConfig`, `OpenWeatherMapConfig`, `OpenRouterConfig`, `DatabaseConfig` and `McpConfig` implement `Debug` by hand with the credential printed as `[REDACTED]`.

**Local development.** `cargo run --features dev-env` loads a `.env` via `dotenvy`; the default build has no dotenv dependency and reads only the process environment.

## Startup Sequence (`backend/src/main.rs`)

1. (`dev-env` only) load `.env`.
2. Initialize `tracing_subscriber::fmt` with `RUST_LOG` (default `info`), so config warnings are logged.
3. `Config::from_env()`, including the `TZ` check; log the resolved zone, offset and local date and time (see Timezone database and `TZ`).
4. `create_pool`: connect to Postgres, retrying with increasing backoff for about a minute while it is unreachable (each failed attempt logged at WARN), then run all pending migrations (below).
5. `ensure_default_profile`: insert a profile from `LAWN_*` when the database has none.
6. `DataSyncService::initialize`: construct the lake, Home Assistant and OpenWeatherMap clients. No network or lake I/O happens here; each source is first touched on demand (Data sync).
7. Construct the OpenRouter client when configured, enabled and keyed; log the model at INFO.
8. Build `AppState` (pool, sync service, OpenRouter, `mcp_enabled`).
9. Assemble the router (below), bind `SERVER_HOST:SERVER_PORT`, and `axum::serve` with graceful shutdown on SIGTERM.

| Startup failure | Behavior |
|---|---|
| `DATABASE_PASSWORD` unset or empty; `MCP_TOKEN` too short; `TZ` set but not a timezone | `Config` error returned from `main`; exit non-zero |
| Postgres unreachable | retried after 2, 4, 8, 16 and 30 s (about a minute, `db::pool::startup_delays`, each failure logged at WARN), so a database that starts a little later (a host reboot, or a run without Compose's `depends_on`) is picked up; still unreachable → error returned from `main`, exit non-zero. Only reachability failures are retried (I/O, pool timeout, TLS, Postgres `57P03` still starting): a wrong password or database name fails at once |
| Migration fails | error returned from `main`; exit non-zero (no retry: a failing migration fails the same way again) |
| Invalid `CORS_ALLOWED_ORIGIN` | panic in `build_cors_layer` |
| Port already bound | `TcpListener::bind` error; exit non-zero |
| Lake, Home Assistant, OWM or OpenRouter unreachable | no effect at startup; degraded per feature at request time |

**Shutdown.** On SIGTERM (or Ctrl-C) the server stops accepting new connections, lets in-flight requests finish, then exits 0 (`axum::serve(...).with_graceful_shutdown`, `main.rs::shutdown_signal`), so `docker stop` or a Compose restart does not cut a request off midway.

## Router Assembly and Middleware

The REST routes (30 paths, 41 method handlers under `/api/v1`, listed in `main.rs` and owned by each feature LLD) are built first, then wrapped, in order, with:

1. `RequestBodyLimitLayer(1 MiB)`.
2. `api::json_errors::with_json_errors` (`backend/src/api/json_errors.rs`): a catch-all route `/api/v1/{*path}` that answers JSON 404 "There is no such API endpoint.", and a `map_response` middleware that rewrites any 4xx/5xx response whose body is not JSON (axum's rejections and the 413 from the body limit) into the `{"error"}` shape (see HTTP Error Contract).
3. The CORS layer (`build_cors_layer`): unset → allow only `http://localhost:<SERVER_PORT>`; `*` → `CorsLayer::permissive()`; otherwise that one origin. Methods GET/POST/PUT/PATCH/DELETE, any header.

After the layers, two things are added that the layers therefore do **not** cover:

- **`/mcp`** (MCP server): merged only when `MCP_TOKEN` is set, guarded by its own bearer middleware. It is server-to-server, so it sits outside CORS. Its internals belong to the MCP server LLD.
- **SPA fallback** (Web UI): when `STATIC_DIR/index.html` exists at startup, `ServeDir(STATIC_DIR)` with `index.html` as the not-found service becomes the router fallback, so any unmatched path outside `/api/v1/` returns the SPA shell; an unknown `/api/v1/...` path has already been answered by the JSON 404 route. Without `index.html` the app runs API-only and logs a warning.

## Database Pool and Migrations (`backend/src/db/pool.rs`)

- `PgPoolOptions::max_connections(DB_MAX_CONNECTIONS or 10).connect_with(...)`, built from discrete `DATABASE_*` fields rather than a URL, over rustls TLS-capable sqlx.
- Migrations live in `backend/src/db/migrations/` as timestamped, forward-only `.sql` files (`YYYYMMDDHHMMSS_name.sql`). `sqlx::migrate!` embeds them at compile time, and `create_pool` runs every pending one at each startup before the server binds. A failure surfaces as `TurfOpsError::Config("Migration failed: …")` and stops startup.
- Each feature LLD owns its tables' schema. This component owns the mechanism: the ordering, startup application, and the rule that a migration ships in the same binary as the code that needs it.
- Queries use sqlx's runtime API (no `query!` macros, no `.sqlx` offline data), so the image and CI build without a database.

## HTTP Error Contract (`backend/src/error.rs`)

All handler errors are `TurfOpsError` and render as JSON `{"error": "<message>"}`.

| Variant | Status | Message to client | Logged |
|---|---|---|---|
| `NotFound(msg)` | 404 | `msg` | no |
| `InvalidData(msg)` | 400 | `msg` | no |
| `Unavailable { origin, outage }` | 503 | a fixed sentence for the source and the outage (below) | WARN with the detail, for `Down(detail)` |
| `Database` violating `applications_unique_turf` / `applications_unique_plant` | 409 | fixed duplicate-application text | no |
| `Database` violating any other unique index | 409 | generic plain-language conflict text (the record conflicts with one that already exists) | no |
| `Config(msg)`, `Internal(msg)` | 500 | `SERVER_ERROR_MESSAGE`: "Something went wrong on the server. Try again; the details are in the server log." | ERROR |
| `Database` (other), `DataLake`, `Http`, `Io`, `Json` | 500 | `SERVER_ERROR_MESSAGE` | ERROR |

`owner_message()` builds the `error` text; `IntoResponse` picks the status and does the logging. `Internal` marks the server's own data being inconsistent (a stored profile row without an id, for example).

**Data-source outages.** `Source` names a source as the owner knows it: `WeatherLake`, `Forecast` (OpenWeatherMap), `PatioSensor` (Home Assistant), `PlanAssistant` (OpenRouter for plant care plans), `ProductAssistant` (OpenRouter answering for a shelf product) and `SoilOutlook` (the 10 cm projection). `Outage` says why: `NotConfigured`, `Disabled`, `Down(detail)` or `NotEnoughData`. Sources build these through `TurfOpsError::unavailable(source, detail)`, `not_configured(source)`, `disabled(source)` and `not_enough_data(source)`; `for_products()` re-labels a `PlanAssistant` outage as `ProductAssistant` so the product endpoints word it for products. Each source and outage pair has one sentence saying what is unavailable, what it affects and what to do, for example "The weather data lake isn't configured, so soil, disease, timing and history data are unavailable. Set DATALAKE_ROOT and restart TurfOps." A `Down` detail (an upstream status and body, a raw model output, a lake task error) is logged and never part of the response.

The duplicate-application text tells the owner how to fix the entry (edit the existing one, or rename the product), so it is used only for the two indexes it describes; the constraint name decides which message applies, and any other unique index gets the generic text rather than advice that does not fit. Requests that fail before a handler runs get the same shape through `with_json_errors`, one plain sentence per status: 400 (malformed JSON, a non-numeric `{id}`) "The request couldn't be read. Check the form and try again.", 405 "That action isn't supported here.", 413 (a body over 1 MiB) "That request is too large to accept.", 415 "The request must be sent as JSON.", 422 "Some fields are missing or not valid. Check the form and try again.", any 5xx the server-error sentence. A response that is already JSON passes through unchanged.

## Health Endpoint (`backend/src/api/health.rs`)

`GET /api/v1/health` always answers **200** with `{status, version, database, datasources, mcp}`. `database` is `SELECT 1` with a 2 s timeout; `status` is `ok` when that succeeds, else `degraded`. `datasources` is Data sync's `ConnectionStatus` (the lake under the legacy key `soildata`). `mcp` is `enabled` or `disabled`. `version` is `CARGO_PKG_VERSION`. No container healthcheck consumes it.

## Logging and Operations

- `tracing` text output to stdout; `RUST_LOG` sets the filter. Startup logs the lake paths, which integrations are configured, the OpenRouter model, whether `/mcp` is mounted, the CORS origin, and the bind address, so a misconfiguration shows on every restart.
- Startup also logs the resolved timezone, offset and local date and time, and warns on every setting that fell back to a default.
- Shipping stdout to a log store is the deployment's concern; the backend assumes nothing about it.
- The container must be able to read the lake as uid 65532.
- No log line may contain a credential. The OpenWeatherMap key travels in the request URL's query string, so its client formats every reqwest error with `without_url()` (`request_error`, `backend/src/datasources/openweathermap.rs`).

## CI, Release and Supply Chain (`.github/workflows/`, `renovate.json`)

| Workflow | Trigger | Gates / output |
|---|---|---|
| `ci.yml` | push / PR to `main` | Backend: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`. Frontend (Node 26): `npm ci`, `npx tsc --noEmit`, `npm run lint`, `npm test` (Vitest). Calls `supply-chain.yml`. |
| `supply-chain.yml` | reusable | `npm ci --ignore-scripts`; `npm audit signatures`; Socket scan of `./frontend` with `--org grecolabs` (skipped when `SOCKET_SECURITY_API_KEY` is absent, as on bot PRs); OSV scan of all lockfiles; `cargo audit`. |
| `docker-publish.yml` | push / PR to `main`, `v*` tags | Runs the supply-chain scan, then builds the image (GHA cache). PRs build without pushing, which is what exercises `npm run build`. Otherwise pushes to GHCR (`latest` on main, semver, `sha`), signs keylessly with cosign, attaches an SPDX SBOM and build-provenance attestations. |
| `ghcr-retention.yml` | Mondays 06:00 UTC / manual | Keeps `latest` plus the 5 newest tagged versions; deletes untagged manifests. |

**Advisory ignores.** `backend/.cargo/audit.toml` (cargo audit) and `backend/osv-scanner.toml` (OSV) name an advisory only while its crate is in `Cargo.lock`, each with a written justification; an ignore whose crate has left the lockfile is deleted, so it cannot silently hide a future advisory of the same ID.

**Renovate.** Runs off-hours on weekdays and anytime on weekends (America/New_York), at most 3 open PRs, creating them immediately and rebasing when behind. Docker images are digest-pinned. Minor and patch updates are grouped, wait 3 days (1 day for common images such as `postgres`) and auto-merge; digest updates auto-merge at any time; majors are labeled `major-update` and wait for review. Weekly lock-file maintenance runs Monday before 4 am; vulnerability alerts are labeled `security`.

**Branch policy.** All changes reach `main` through a PR; the owner merges.

## Decisions & Alternatives

| Decision | Chosen | Alternatives Considered | Rationale |
|---|---|---|---|
| Runtime base | Chainguard `glibc-dynamic` distroless | Debian slim; Alpine/musl; scratch | No shell or package manager, non-root by default, small CVE surface; glibc avoids musl build friction with bundled DuckDB. [inferred] |
| C++ runtime for DuckDB | Static `-static-libstdc++ -static-libgcc` | Ship `libstdc++.so.6` in the image; a runtime base that has it | The distroless runtime lacks libstdc++; linking it statically keeps the runtime glibc-only without widening the image. |
| Weather lake delivery | Read-only bind mount at `/data`, paths derived from `DATALAKE_ROOT` | Weather HTTP API; Postgres copy | Local parquet is fast and always current; read-only protects the pipeline's data. Overrides exist for a different layout. |
| Timezone | Ship `/usr/share/zoneinfo`; deployment sets `TZ` to the lawn's timezone (Compose default `America/New_York`); the app's `Clock`, resolved once from `TZ`, is the one "today" | Compute all dates in UTC; a station-local date from the lake's UTC offset; the viewer's browser date; hard-code the zone in the image | Every agronomic date is a lawn-local calendar day. One configured clock gives every component the same day. Lake timestamps are UTC, so the UTC date would be wrong every evening. The image cannot know the lawn's location, so the zone belongs to the deployment. |
| Required DB password syntax | `${DB_PASSWORD:?}` | `${DB_PASSWORD:?message}`; a default password | A missing password must stop Compose; the message form trips the ggshield secret scanner. |
| Optional keys | Empty or whitespace = not configured | Treat empty as a value; separate `*_ENABLED` only | Compose passes unset `${KEY:-}` as `""`; treating it as configured would start clients with blank credentials. [inferred] |
| Container health check | The binary probes its own `/api/v1/health` (`--healthcheck`), declared in the image | `curl` in the runtime; a Compose-only check; no check | The distroless runtime has no shell or curl and should stay that way; an image-level check covers every deployment of the image, not just the example Compose stack. |
| What the startup retry waits out | Only reachability failures | Every connect error | A wrong password or database name will not fix itself; waiting a minute on it only delays the error the owner needs to see. |
| Config loading | `config::load::from_vars(lookup)` returns the config and its warnings; `from_env` passes the process environment and logs them | Reading `std::env` directly | Tests can feed any combination of settings without touching the process environment, which parallel tests share. |
| Malformed security config | Refuse to start (`MCP_TOKEN` < 32, bad CORS origin) | Warn and continue | A short token or wrong origin would silently weaken an open endpoint (MCP plan, `docs/plans/ai-assistant.md`). |
| Other malformed values | Warn, naming the setting and the default, then fall back | Refuse to start; fall back silently | Keeps the app up on a typo in a non-security setting, while the warning makes the typo visible on the next restart. |
| Empty `DATABASE_PASSWORD` | Refuse to start | Connect with a blank password | A blank password is never intended; failing at startup with the setting's name beats an authentication error from Postgres, and runs without Compose's `:?` guard get the same protection. |
| `TZ` handling | Log zone, offset and local time; warn when unset; refuse when set but unresolvable | Refuse to start when unset; trust `TZ` silently | A local `cargo run` without `TZ` correctly uses the OS zone, so unset cannot be fatal; a set but unresolvable value is always a mistake and would otherwise run on UTC without a word. |
| `*_ENABLED` flags | `true`, `1`, `yes`, case-insensitive | Exact `true` only | `TRUE` or `1` turning a feature off is a surprising failure; these are the common spellings of "on". |
| Role of `docker-compose.yml` | An example any user can run to try the project | The definition of the owner's deployment | Keeps the repo usable by anyone; an environment-specific deploy belongs with that environment, not in the repo. |
| Example Postgres port | Published on `127.0.0.1:5433` only | All interfaces; not published | Local `cargo run` against the Compose database keeps working without exposing the database to the network. |
| Postgres connection at startup | Retry with backoff for about a minute, then exit | Fail on the first error; retry forever | Rides out a database that starts a little later without relying on a restart policy, while a wrong host or password still surfaces as an exit within a minute. |
| npm lifecycle scripts in the image build | `npm ci --ignore-scripts` | Run install scripts | Matches the supply-chain scan's policy; install scripts are a common malware vector and no dependency needs one. |
| Migrations | Embedded, run at every startup before bind | Separate migration job; manual `sqlx migrate run` | One binary is always in step with its schema; single-instance deploy has no migration race. [inferred] |
| Query style | sqlx runtime queries | `query!` compile-time macros | Builds need no live DB or offline metadata in CI or Docker. [inferred] |
| Error exposure | Every `error` is written for the owner: 4xx carry the handler's message, a 503 a fixed sentence per source and outage, a 500 one generic sentence; upstream and internal details only in logs | Echo all error text; pass the source's own error string through | Hides DB, upstream and internal details from clients, and the owner learns what is unavailable and what to do instead of reading a provider's response body. |
| Unique-violation message | Duplicate-application text only for the two application indexes; generic conflict text otherwise | One message for every unique violation | The duplicate text gives fix-it advice specific to the application log, which would mislead for any other record. |
| Health semantics | Always 200 with `ok` / `degraded` | 503 when the DB is down | Clients get a readable status body; integrations are reported, not fatal. [inferred] |
| Release integrity | cosign keyless signing, SBOM, provenance on every pushed image | Unsigned images | Verifiable supply chain for the deployed image. [inferred] |
| Socket scan invocation | `--org grecolabs`, self-skip without the secret | Interactive org selection; required secret | Without `--org` the CLI prompts and hangs CI; bot PRs have no secrets, so a required secret would fail them outright. |
| Dependency currency | Renovate auto-merges grouped minor/patch after a soak; majors reviewed | Manual updates; auto-merge everything | Stays current without unreviewed breaking changes. [inferred] |

## Open Questions & Future Decisions

### Resolved
1. ✅ The image contains no `.env`: `.dockerignore` excludes `.env` / `.env.*`, and only `backend/src`, the Cargo manifests, `frontend/` and two docs are copied.
2. ✅ `/mcp` is unmounted, not merely unauthenticated, when `MCP_TOKEN` is empty.

### Deferred
1. **No panic hook.** There is none beyond the default. `build_cors_layer` (`main.rs`) and the OWM client builder (`datasources/openweathermap.rs`) use `expect`, so a bad CORS origin ends startup in a panic, not in the `Config` error every other refused setting produces.
2. **Settings outside the loader.** `STATIC_DIR` is read in `main.rs`, not in `config::load`, and an OWM key with unset or unparseable coordinates forecasts for 0°, 0° (the bad value is warned about, the 0° default is not).
3. **`OPENROUTER_MODEL` is not validated.** An id OpenRouter does not know surfaces only when the assistant is first called (the default, `anthropic/claude-haiku-4.5`, is valid).
4. **CI does not build the frontend.** `ci.yml` type-checks, lints and runs the Vitest suite; only the PR image build in `docker-publish.yml` runs `npm run build`.
5. **`.dockerignore` patterns are root-anchored.** `target/`, `src/` and `*.md` match only top-level paths, so `backend/target` (tens of GB locally) is not excluded. BuildKit transfers only paths the Dockerfile copies, which keeps this harmless today. Widening `*.md` to `**/*.md` would break the MCP docs copy.
6. **`main.rs` is 353 lines,** over the 300-line limit; the route table is the natural split (for example `api::router()`).
7. **The example stack's database user is fixed.** The `db` service always creates `turfops`, while `app` takes `${DATABASE_USER:-turfops}`; a `.env` that sets `DATABASE_USER` (Compose reads `.env` automatically) makes the app authenticate as a user the database never created.

## References

- `Dockerfile`, `docker-compose.yml`, `.dockerignore`, `.gitignore`, `renovate.json`
- `backend/src/main.rs`, `backend/src/config.rs`, `backend/src/error.rs`, `backend/src/api/json_errors.rs`, `backend/src/db/pool.rs`, `backend/src/db/migrations/`, `backend/src/api/health.rs`, `backend/src/mcp/config.rs`, `backend/src/mcp/resources.rs`
- `backend/Cargo.toml`, `backend/.cargo/audit.toml`, `backend/osv-scanner.toml`, `backend/.env.example`
- `.github/workflows/ci.yml`, `supply-chain.yml`, `docker-publish.yml`, `ghcr-retention.yml`
- `README.md` (Environment Variables, Docker Build, local development)
- `docs/plans/ai-assistant.md` (MCP token and mounting rules)
- Boundaries: Weather data access (lake paths, client construction), Data sync (`ConnectionStatus`, on-demand refresh), MCP server (`/mcp` router and auth), Web UI (SPA served from `STATIC_DIR`)
