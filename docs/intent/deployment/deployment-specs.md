# Deployment, Configuration & Operations — EARS Specs

Design: [deployment-design.md](deployment-design.md). Prefix `DEPLOY`. Facets: `IMAGE` (the Docker image), `COMPOSE` (the example Compose stack), `CONFIG` (environment configuration surface), `TZ` (the one "today" clock), `START` (startup sequence and migrations), `HTTP` (router assembly, middleware and the HTTP error contract), `HEALTH` (the health endpoint), `LOG` (logging and secret hygiene), `CI` (CI, release and supply chain).

"Credential" below means any of `DATABASE_PASSWORD` / `DB_PASSWORD`, `HA_TOKEN`, `OWM_API_KEY`, `OPENROUTER_API_KEY` and `MCP_TOKEN`. Lake paths, the station filter and the Home Assistant / OpenWeatherMap configured-when rules are specified by Weather sources (`WX-SRC-CFG-001`…`003`).

## Container image

- [x] **DEPLOY-IMAGE-001**: The Docker image shall be built in three stages (frontend build on Node, backend build on Rust, distroless runtime), with every base image pinned by digest.
- [x] **DEPLOY-IMAGE-002**: The runtime image shall contain only the release backend binary, the built SPA at `/app/static` and the timezone database at `/usr/share/zoneinfo`, with no shell, package manager or C++ runtime library.
- [x] **DEPLOY-IMAGE-003**: The image's backend build shall link libstdc++ and libgcc statically, so the binary's only dynamic dependency is glibc (the distroless runtime ships no `libstdc++.so.6`, which bundled DuckDB needs).
- [x] **DEPLOY-IMAGE-004**: The image's backend build stage shall copy `docs/timing-windows.md` and `docs/agronomy-methods.md` to `/docs/` before compiling, so the MCP server's embedded methodology resources resolve, and `.dockerignore` shall leave `docs/*.md` in the build context.
- [x] **DEPLOY-IMAGE-005**: The runtime image shall run the backend as the non-root uid 65532, with the binary and static files owned by 65532:65532.
- [x] **DEPLOY-IMAGE-006**: The runtime image shall set `STATIC_DIR=/app/static`, expose port 3000 and use the backend binary as its entrypoint.
- [x] **DEPLOY-IMAGE-007**: The Docker build context shall exclude `.env` and every `.env.*` except `.env.example`, so no image contains an environment file.
- [x] **DEPLOY-IMAGE-008**: The app container (image or Compose `app` service) shall declare a health check that probes `GET /api/v1/health` and marks the container unhealthy when it does not answer, using a probe that works without a shell or curl in the distroless runtime.
- [x] **DEPLOY-IMAGE-009**: The runtime image shall declare `USER 65532:65532` explicitly, so the backend runs as that non-root uid even if the base image's default user changes.
- [x] **DEPLOY-IMAGE-010**: The image's frontend build stage shall install dependencies with `npm ci --ignore-scripts` (no package lifecycle scripts, matching the supply-chain scan), unless a dependency genuinely needs an install script, in which case the Dockerfile shall say which one and why.

## Compose stack

- [x] **DEPLOY-COMPOSE-001**: The example Compose stack (`docker-compose.yml`) shall run two services, `db` (`postgres:18-alpine` pinned by digest, database and user `turfops`, data on the named volume `turfops_data`) and `app` (built from the Dockerfile), both with `restart: unless-stopped`.
- [x] **DEPLOY-COMPOSE-002**: If `DB_PASSWORD` is unset or empty, then Compose shall refuse to start the stack; the one value shall feed both the `db` service's `POSTGRES_PASSWORD` and the `app` service's `DATABASE_PASSWORD`, written in the bare `${DB_PASSWORD:?}` form with no message (the message form trips the secret scanner).
- [x] **DEPLOY-COMPOSE-003**: The Compose `app` service shall start only after the `db` service's health check (`pg_isready -U turfops`, every 5 s, 5 retries) passes.
- [x] **DEPLOY-COMPOSE-004**: The Compose `app` service shall mount the NOAA weather lake read-only from `${DATALAKE_HOST_PATH:-/data}` at `/data`.
- [x] **DEPLOY-COMPOSE-005**: The Compose `app` service shall pass `TZ` to the backend, defaulting to `America/New_York` when the host leaves it unset (a user sets it to their lawn's timezone).
- [x] **DEPLOY-COMPOSE-006**: The Compose `app` service shall pass every optional setting as `${KEY:-}` or `${KEY:-default}`, so a variable the host leaves unset reaches the backend as an empty string or the stated default; it shall fix `SERVER_HOST=0.0.0.0`, `SERVER_PORT=3000` and `STATIC_DIR=/app/static`, and default `RUST_LOG` to `info`.
- [x] **DEPLOY-COMPOSE-007**: The Compose stack shall publish the app on host port 3000, and PostgreSQL on host port 5433 bound to the host's loopback interface only (`127.0.0.1:5433:5432`), so a local `cargo run` can use the Compose database but other machines on the network cannot reach it.
- [x] **DEPLOY-COMPOSE-009**: The Compose `db` service shall mount its named volume `turfops_data` where the PostgreSQL 18 image keeps its data (the volume root `/var/lib/postgresql`, under which the image stores the cluster in `18/docker`), so a fresh `docker compose up` initializes the database and keeps it across container recreation.

## Configuration

- [x] **DEPLOY-CONFIG-001**: The backend shall read its configuration from environment variables once at startup, giving every setting a default except `DATABASE_PASSWORD`.
- [x] **DEPLOY-CONFIG-002**: If `DATABASE_PASSWORD` is unset or empty at startup, then the backend shall stop with a configuration error naming `DATABASE_PASSWORD` and exit non-zero.
- [x] **DEPLOY-CONFIG-003**: Where `OPENROUTER_API_KEY`, `MCP_TOKEN` or `CORS_ALLOWED_ORIGIN` is empty or whitespace-only, the backend shall treat that setting as not configured.
- [x] **DEPLOY-CONFIG-004**: The backend shall construct the OpenRouter client only when `OPENROUTER_API_KEY` is non-blank and `OPENROUTER_ENABLED` is on (DEPLOY-CONFIG-013; on by default), with the model from `OPENROUTER_MODEL` and the base URL from `OPENROUTER_BASE_URL` (default `https://openrouter.ai/api/v1`).
- [x] **DEPLOY-CONFIG-007**: If `DATABASE_PORT`, `SERVER_PORT` or `LAWN_GRASS_TYPE` cannot be parsed, then the backend shall log a warning naming the value and use its default (5432, 3000, TallFescue).
- [x] **DEPLOY-CONFIG-008**: The backend shall size its PostgreSQL pool from `DB_MAX_CONNECTIONS`, defaulting to 10.
- [x] **DEPLOY-CONFIG-009**: When the backend starts with no lawn profile in the database, it shall create one from `LAWN_NAME`, `LAWN_GRASS_TYPE`, `LAWN_USDA_ZONE`, `LAWN_SOIL_TYPE`, `LAWN_SIZE_SQFT` and `LAWN_IRRIGATION_TYPE`; once a profile exists, the backend shall ignore the `LAWN_*` settings.
- [x] **DEPLOY-CONFIG-010**: Every configuration struct that holds a credential shall print it as `[REDACTED]` in its `Debug` output.
- [x] **DEPLOY-CONFIG-011**: Where the backend is built with the `dev-env` feature, it shall load a `.env` file before reading configuration; the default build shall read only the process environment.
- [x] **DEPLOY-CONFIG-012**: If any configuration setting is set to a non-empty value the backend cannot use (for example an unparseable `SERVER_HOST`, `DB_MAX_CONNECTIONS`, `OWM_LATITUDE` / `OWM_LONGITUDE` or `LAWN_SIZE_SQFT`, an unknown `LAWN_SOIL_TYPE`, `LAWN_IRRIGATION_TYPE` or `HA_TEMPERATURE_UNIT`, or an invalid `RUST_LOG` filter), then the backend shall log a warning naming the setting and the default it uses instead, never falling back silently.
- [x] **DEPLOY-CONFIG-013**: When `OWM_ENABLED` or `OPENROUTER_ENABLED` is set, the backend shall treat `true`, `1` or `yes`, in any letter case, as on and any other value as off; when unset, the flag shall be on.

## Timezone ("today")

- [x] **DEPLOY-TZ-001**: The runtime image shall ship the timezone database at `/usr/share/zoneinfo`, so the backend's local clock (`chrono::Local`) honors `TZ`.
- [x] **DEPLOY-TZ-002**: The runtime image shall set no `TZ` of its own; the deployment shall supply the lawn's timezone, which every component uses as its one "today" clock.
- [x] **DEPLOY-TZ-003**: When the backend starts, it shall log at INFO the `TZ` value, the UTC offset it resolved to and the resulting local date and time, so a wrong clock shows in the startup log.
- [x] **DEPLOY-TZ-004**: If `TZ` is unset or empty at startup, then the backend shall log a warning that every date gate will follow the host's default zone (UTC in the image) and that `TZ` must be set to the lawn's timezone.
- [x] **DEPLOY-TZ-005**: If `TZ` is set to a value that does not resolve to a timezone (for example an IANA name missing from the zoneinfo database), then the backend shall stop with a configuration error naming `TZ` and exit non-zero, rather than fall back silently to UTC.

## Startup and migrations

- [x] **DEPLOY-START-001**: When the backend starts, it shall initialize logging, then read configuration, then connect to PostgreSQL and run migrations, then ensure a lawn profile exists, then construct the data-source and OpenRouter clients, then assemble the router and bind `SERVER_HOST:SERVER_PORT`.
- [x] **DEPLOY-START-002**: When the backend starts, it shall apply every pending migration embedded in the binary (timestamped, forward-only `.sql` files from `backend/src/db/migrations/`) before binding the listener, so a binary always runs against the schema it ships with.
- [x] **DEPLOY-START-003**: If PostgreSQL is still unreachable when the startup connection retries (DEPLOY-START-007) run out, or a migration fails at startup, then the backend shall exit non-zero without binding the listener.
- [x] **DEPLOY-START-004**: When the backend starts, it shall perform no network or lake I/O for the weather lake, Home Assistant, OpenWeatherMap or OpenRouter, so an unreachable integration never prevents startup.
- [x] **DEPLOY-START-005**: The backend shall issue database queries through sqlx's runtime API (no compile-time-checked query macros or offline query metadata), so the image and CI build without a database.
- [x] **DEPLOY-START-006**: When the running backend receives SIGTERM, it shall stop accepting new connections, let in-flight requests finish, and then exit.
- [x] **DEPLOY-START-007**: If PostgreSQL is unreachable at startup, then the backend shall retry the connection with increasing backoff for about one minute, logging each failed attempt at WARN, before giving up.

## Router, middleware and error contract

- [x] **DEPLOY-HTTP-001**: The backend shall apply a 1 MiB request-body limit to every `/api/v1` route (the CORS policy is UI-SRV-004 to 006; `/mcp` mounting and its token rules are MCP-CFG-001 to 005).
- [x] **DEPLOY-HTTP-004**: Where `STATIC_DIR/index.html` exists at startup, the backend shall serve `STATIC_DIR` as the router fallback, answering any unmatched path with `index.html` (client-side routing); otherwise it shall run API-only and log a warning.
- [x] **DEPLOY-HTTP-005**: The backend shall render every API handler error as JSON `{"error": "<message>"}` with status 404 for not-found, 400 for invalid data, 503 for an unavailable data source, 409 for a conflict with an existing record, and 500 for every other error.
- [x] **DEPLOY-HTTP-006**: When an API write violates an application-log duplicate guard (`applications_unique_turf` / `applications_unique_plant`), the backend shall answer 409 with "An identical entry (same type, product and date) is already logged. Edit the existing entry, or change the product name if this was a different product."
- [x] **DEPLOY-HTTP-007**: If an API handler fails with a configuration, database, data-lake, HTTP-client, I/O or JSON error, then the backend shall log the error's details at ERROR and keep them out of the response body.
- [x] **DEPLOY-HTTP-008**: When the backend answers 500, the `error` message shall be plain language that says something went wrong on the server and what to do (try again; the details are in the server log).
- [x] **DEPLOY-HTTP-009**: When the backend answers 503 for an unavailable data source, the `error` message shall be written for the owner, naming the source that is unavailable, and shall carry no upstream response body, request URL, credential or internal error text; those details shall go to the log.
- [x] **DEPLOY-HTTP-010**: If an `/api/v1` request cannot be extracted (malformed JSON body, a path id that is not a number, a body over 1 MiB), then the backend shall answer with the JSON `{"error"}` shape and a plain-language message saying what was wrong with the request, not the framework's plain-text rejection.
- [x] **DEPLOY-HTTP-011**: If an API write violates a unique index other than the application-log duplicate guards (`applications_unique_turf` / `applications_unique_plant`), then the backend shall answer 409 with a generic plain-language message that the record conflicts with one that already exists, never the duplicate-application text.

## Health endpoint

- [x] **DEPLOY-HEALTH-001**: `GET /api/v1/health` shall always answer 200 with `{status, version, database, datasources, mcp}`.
- [x] **DEPLOY-HEALTH-002**: When `GET /api/v1/health` is called, the backend shall report `database` true when `SELECT 1` succeeds within 2 s, and `status` `ok` when it does, otherwise `degraded`.
- [x] **DEPLOY-HEALTH-003**: `GET /api/v1/health` shall report `datasources` as data sync's connection status (the weather lake under the legacy key `soildata`), `mcp` as `enabled` or `disabled` by whether `/mcp` is mounted, and `version` as the backend crate version.

## Logging

- [x] **DEPLOY-LOG-001**: The backend shall write text logs to stdout, filtered by `RUST_LOG` (default `info`; an invalid filter falls back to `info`), with logging initialized before configuration is read so configuration warnings are logged.
- [x] **DEPLOY-LOG-002**: When the backend starts, it shall log at INFO the weather lake paths, which integrations are configured, the OpenRouter model, whether `/mcp` is mounted, the CORS origin and the bind address.
- [x] **DEPLOY-LOG-003**: No log line and no HTTP response body produced by the backend shall contain a credential.

## CI, release and supply chain

- [x] **DEPLOY-CI-001**: When code is pushed to `main` or a pull request targets `main`, CI shall run `cargo fmt --check`, `cargo clippy -- -D warnings` and `cargo test` on the backend, and `npm ci` and `npm run lint` on the frontend (Node 26).
- [x] **DEPLOY-CI-002**: When code is pushed to `main` or a pull request targets `main`, CI shall run the supply-chain scan: frontend install with `--ignore-scripts`, `npm audit signatures`, a Socket scan of `./frontend` with `--org grecolabs`, an OSV scan of every lockfile, and `cargo audit`.
- [x] **DEPLOY-CI-003**: Where the `SOCKET_SECURITY_API_KEY` secret is absent (bot-authored pull requests), the supply-chain scan shall skip the Socket step and still run its other checks.
- [x] **DEPLOY-CI-004**: When a pull request targets `main`, the image workflow shall build the Docker image (including the frontend production build) after the supply-chain scan passes, without pushing it.
- [x] **DEPLOY-CI-005**: When code is pushed to `main` or a `v*` tag is pushed, the image workflow shall push the image to `ghcr.io/tgrecojr/turfops` tagged by commit sha, by semver for tags and `latest` for `main`, sign it keylessly with cosign, and attach an SPDX SBOM attestation and a build-provenance attestation.
- [x] **DEPLOY-CI-006**: Every Monday at 06:00 UTC, and on manual dispatch, the retention workflow shall keep `latest` and the 5 newest tagged image versions in GHCR and delete untagged manifests.
- [x] **DEPLOY-CI-007**: Renovate shall pin Docker images by digest; group minor and patch updates and auto-merge them after a 3-day soak (1 day for common images such as `postgres`); auto-merge digest updates at any time; label major updates `major-update` and leave them for review; keep at most 3 open PRs; run lock-file maintenance Mondays before 4 am; and label vulnerability alerts `security`.
- [x] **DEPLOY-CI-008**: Every change shall reach `main` through a pull request that the owner merges.
- [x] **DEPLOY-CI-009**: The supply-chain scan's advisory ignore lists (`backend/.cargo/audit.toml` for `cargo audit`, `backend/osv-scanner.toml` for OSV) shall name an advisory only while its affected crate is in `backend/Cargo.lock`, each with a written justification.
