# TurfOps Comprehensive Code Review

**Date:** 2026-03-15
**Scope:** Full codebase (backend, frontend, infrastructure)
**Purpose:** Identify inefficiencies, security weaknesses, and improvement opportunities

---

## Implementation Tracker

| ID | Issue | Status | PR/Commit |
|----|-------|--------|-----------|
| C1 | `.unwrap()` panics on `profile.id` in 5 API handlers | Done | |
| C2 | Chained `.unwrap()` on serde enum serialization (6 locations) | Done | |
| C3 | `partial_cmp().unwrap()` can panic on NaN | Done | |
| C4 | `assess_day_quality` unwrap on forecast | Done | |
| H1 | `CorsLayer::permissive()` in production | Done | |
| H3 | DB/SoilData passwords exposed via `#[derive(Debug)]` | Done | |
| H4 | Mutex contention serializes all data-sync requests | Done | |
| H5 | CI workflow has no tests or linting | Done | |
| H6 | Missing `.env.example` file | Done | |
| H8 | `null` keys on React list items | Done | |
| H2 | No authentication/authorization | Future | |
| H7 | Accessibility regressions in frontend | Future | |
| M1 | `environmental_cache` grows unbounded | Future | |
| M2 | Calendar fetches all applications | Future | |
| M3 | No pagination on application listing | Future | |
| M4 | Migration index bug | Future | |
| M5 | Polling continues in background tabs | Future | |
| M6 | No request cancellation on unmount | Future | |
| M7 | Delete/action buttons have no loading guard | Future | |
| M8 | Duplicated gauge configuration | Future | |
| M9 | Duplicated style objects across pages | Future | |
| M10 | DB credentials in connection string without encoding | Future | |
| M11 | Secrets as plain env vars in Docker | Future | |
| M12 | `SOILDATA_DB_PASSWORD` defaults to empty string | Future | |
| M13 | `--legacy-peer-deps` suppresses dependency conflicts | Future | |
| L1–L14 | Low priority items | Future | |

---

## Table of Contents

1. [Critical Issues (Fix Immediately)](#1-critical-issues)
2. [High Priority (Fix Soon)](#2-high-priority)
3. [Medium Priority (Plan to Address)](#3-medium-priority)
4. [Low Priority (Nice to Have)](#4-low-priority)
5. [Summary](#5-summary)

---

## 1. Critical Issues

### C1. Server Panics from `.unwrap()` on `profile.id`

**Files:** `backend/src/api/applications.rs:25,75`, `backend/src/api/calendar.rs:36`, `backend/src/api/dashboard.rs:29`, `backend/src/api/recommendations.rs:26`

Five API handlers call `profile.id.unwrap()` where `profile.id` is `Option<i64>`. If the database ever returns a profile without an ID, the server panics and crashes the process.

**Recommendation:** Replace `.unwrap()` with proper error handling:
```rust
let profile_id = profile.id.ok_or_else(|| AppError::Internal("Profile missing ID".into()))?;
```

### C2. Server Panics from Chained `.unwrap()` in Enum Serialization

**File:** `backend/src/db/queries.rs:22-26, 60-63, 67-73, 75-79, 123-127, 170-174`

Pattern like `serde_json::to_value(profile.grass_type).unwrap().as_str().unwrap()` appears 6 times. If serialization produces a non-string JSON value, `.as_str()` returns `None` and panics.

**Recommendation:** Use `serde_json::to_string()` directly, or implement `Display` on the enums and use `.to_string()`.

### C3. Server Panics from `partial_cmp().unwrap()` on NaN

**File:** `backend/src/datasources/openweathermap.rs:214, 221, 231, 251`

`f64::partial_cmp()` returns `None` for NaN values. Weather API data could contain NaN, causing a server panic.

**Recommendation:** Use `.unwrap_or(std::cmp::Ordering::Equal)` (already done correctly in `forecast.rs:61`).

### C4. Server Panic in `assess_day_quality`

**File:** `backend/src/logic/rules/application_window.rs:135`

`env.forecast.as_ref().unwrap()` panics if forecast is `None`. The caller checks, but the method itself doesn't enforce the invariant.

**Recommendation:** Accept `&WeatherForecast` directly or return an error.

---

## 2. High Priority

### H1. Overly Permissive CORS

**File:** `backend/src/main.rs:87`

`CorsLayer::permissive()` allows any origin, method, and header. Combined with no authentication (H2), any website can make API calls to this server.

**Recommendation:** Restrict CORS to the expected frontend origin. For Docker deployment, this could be the container's hostname or `localhost:3000`.

### H2. No Authentication or Authorization

**Scope:** All API endpoints

The entire API is unauthenticated. Any network-reachable client can read/write data, delete applications, and trigger external API calls. The delete endpoint (`api/applications.rs:96-102`) doesn't even verify ownership.

**Recommendation:** For a home-network deployment, consider at minimum:
- A simple API key/token mechanism
- Binding to localhost only unless explicitly configured otherwise

### H3. Database Passwords Exposed in Debug Output

**File:** `backend/src/config.rs:108-115` (DatabaseConfig), `backend/src/config.rs:30` (SoilDataConfig)

Both structs derive `Debug` which prints passwords in logs. `HomeAssistantConfig` and `OpenWeatherMapConfig` correctly have custom `Debug` impls that redact secrets.

**Recommendation:** Add custom `Debug` impls that redact `password` fields, matching the pattern already used for HA and OWM configs.

### H4. Mutex Contention Serializes All Data Requests

**Files:** `backend/src/state.rs:14`, `backend/src/api/dashboard.rs:33-37`, `backend/src/api/health.rs:26-27`

`DataSyncService` is behind `Arc<Mutex>`. The dashboard and health endpoints hold this mutex during network I/O (up to 3 external service calls). This serializes all concurrent requests — a slow external service blocks everything.

**Recommendation:**
- Replace outer `Mutex` with `RwLock` (or `tokio::sync::RwLock`)
- Move connection checks out of the mutex-holding scope
- Remove redundant inner `Arc<RwLock>` wrappers on `current_summary` and `current_forecast` (`data_sync.rs:22-23`)

### H5. CI Workflow Has No Tests or Linting

**File:** `.github/workflows/docker-publish.yml`

The only CI job builds and pushes a Docker image. No `cargo test`, `cargo clippy`, `cargo fmt --check`, `npm run lint`, or `npx tsc --noEmit`. Broken code merges undetected.

**Recommendation:** Add a CI job that runs before the Docker build:
```yaml
- run: cargo fmt --check
- run: cargo clippy -- -D warnings
- run: cargo test
- run: cd frontend && npx tsc --noEmit
- run: cd frontend && npm run lint
```

### H6. Missing `.env.example` File

**Scope:** Project root

CLAUDE.md references `backend/.env.example` but no such file exists. New developers have no template for the 20+ required environment variables defined in `docker-compose.yml`.

**Recommendation:** Create `.env.example` with all variables and placeholder values.

### H7. Accessibility Regressions in Frontend

| Issue | File | Impact |
|-------|------|--------|
| `aria-current={undefined}` removes screen reader page indicator | `frontend/src/components/Layout.tsx:25` | Screen readers can't identify active page |
| Gauge has no ARIA meter role | `frontend/src/components/Gauge.tsx` | Gauge values invisible to screen readers |
| Calendar days not keyboard-accessible | `frontend/src/pages/Calendar.tsx:125` | Keyboard users can't interact |
| Recommendation items not keyboard-accessible | `frontend/src/pages/Recommendations.tsx:75` | Keyboard users can't interact |
| Navigation arrows have no aria-label | `frontend/src/pages/Calendar.tsx:86,92` | Screen readers announce meaningless text |
| Form labels not associated with inputs | `frontend/src/pages/Settings.tsx`, `Applications.tsx` | Inputs may not be announced correctly |
| Connection dots use color only for status | `frontend/src/pages/Dashboard.tsx:184-196` | Color-blind users can't distinguish status |

### H8. `null` Keys on React List Items

**Files:** `frontend/src/pages/Dashboard.tsx:156`, `frontend/src/pages/Applications.tsx:112`

`app.id` is typed as `number | null`. When `id` is `null`, multiple rows share the same key, breaking React reconciliation and potentially causing wrong-row deletions.

**Recommendation:** Use a fallback key: `key={app.id ?? `temp-${index}`}` or ensure IDs are always present.

---

## 3. Medium Priority

### M1. `environmental_cache` Table Grows Unbounded

**File:** `backend/src/db/queries.rs:158-193`

A new row is inserted every 5 minutes on sensor refresh, with no retention policy. Over a year, this is ~100k rows.

**Recommendation:** Add a cleanup query (e.g., delete rows older than 90 days) that runs periodically, or add a migration with a scheduled trigger.

### M2. Calendar Endpoint Fetches All Applications

**File:** `backend/src/api/calendar.rs:36`

Loads all applications for the profile, then filters by month in Rust. Grows linearly with application history.

**Recommendation:** Add date range parameters to the SQL query.

### M3. No Pagination on Application Listing

**Files:** `backend/src/api/applications.rs:20-37`, `backend/src/db/queries.rs:96-107`

All applications are returned in a single response. Over years of use this could be hundreds of records.

**Recommendation:** Add `LIMIT`/`OFFSET` or cursor-based pagination.

### M4. Migration Index Bug

**File:** Migration `20240101000002:32`

`CREATE INDEX IF NOT EXISTS idx_environmental_cache_timestamp` silently keeps the old ASC index instead of replacing it with the intended DESC index. The `IF NOT EXISTS` prevents the new index from being created.

**Recommendation:** Add `DROP INDEX IF EXISTS idx_environmental_cache_timestamp;` before the `CREATE INDEX`.

### M5. Polling Continues in Background Tabs

**Files:** `frontend/src/pages/Dashboard.tsx:8,29`, `frontend/src/pages/Environmental.tsx:9,31`

30-second `setInterval` fires even when the browser tab is hidden, wasting network requests.

**Recommendation:** Use the Page Visibility API to pause polling when the tab is hidden.

### M6. No Request Cancellation on Component Unmount

**Files:** `frontend/src/pages/Dashboard.tsx:27-31`, `frontend/src/pages/Environmental.tsx:29-33`

Polling `useEffect` clears the interval but doesn't abort in-flight requests. `Calendar.tsx` correctly uses a `cancelled` flag.

**Recommendation:** Use `AbortController` in the `useEffect` cleanup, or add a mounted/cancelled guard.

### M7. Delete/Action Buttons Have No Loading Guard

**Files:** `frontend/src/pages/Applications.tsx:49`, `frontend/src/pages/Recommendations.tsx:31`

Rapid clicks can fire duplicate DELETE or PATCH requests.

**Recommendation:** Add a loading state that disables the button while the request is in flight.

### M8. Duplicated Gauge Configuration

**Files:** `frontend/src/pages/Dashboard.tsx:70-130`, `frontend/src/pages/Environmental.tsx:84-127`

The exact same 4 gauge components with identical thresholds are copy-pasted between two pages.

**Recommendation:** Extract gauge configs into a shared constant or component.

### M9. Duplicated Style Objects Across Pages

**Files:** All page components (`Dashboard`, `Applications`, `Calendar`, `Environmental`, `Recommendations`)

Nearly identical `styles.badge`, `styles.table`, `styles.th`, `styles.td`, `styles.error`, `styles.card` objects are repeated in every page.

**Recommendation:** Extract into a shared styles module or use CSS modules/classes.

### M10. Database Credentials in Connection String Without Encoding

**File:** `backend/src/config.rs:41-44, 119-122`

`format!()` interpolates user/password directly into the connection URL. Special characters in passwords (`@`, `#`, `:`, etc.) will break or corrupt the connection string.

**Recommendation:** Use `PgConnectOptions` builder or `urlencoding::encode()` on the credentials.

### M11. Secrets Passed as Plain Environment Variables in Docker

**File:** `docker-compose.yml:29-67`

`DB_PASSWORD`, `HA_TOKEN`, `OWM_API_KEY` are plain env vars visible via `docker inspect`.

**Recommendation:** Use Docker Compose `secrets:` with file-based secrets.

### M12. `SOILDATA_DB_PASSWORD` Defaults to Empty String

**File:** `docker-compose.yml:57`

`${SOILDATA_DB_PASSWORD:-}` silently defaults to empty if unset.

**Recommendation:** Fail loudly — remove the default or validate at startup.

### M13. `--legacy-peer-deps` Suppresses Real Dependency Conflicts

**File:** `Dockerfile:5`

This flag masks ESLint v10 peer dependency issues rather than resolving them.

**Recommendation:** Resolve the underlying peer dependency conflict so the flag can be removed.

---

## 4. Low Priority

### L1. Magic Numbers Throughout Rules and Frontend

**Backend:** All 18 rule files use inline numeric thresholds (85.0, 0.10, 0.40, 50.0, 60.0, etc.)
**Frontend:** Poll intervals (`30_000`), API timeout (`15_000`), gauge ranges, sidebar width (`220`)

**Recommendation:** Extract into named constants. Domain thresholds (agronomic values) especially benefit from being centralized and configurable.

### L2. Custom `from_str` Instead of `FromStr` Trait

**Files:** `backend/src/models/lawn_profile.rs:39,86,121`, `application.rs:40`, `frac_class.rs:54`

Inherent `fn from_str` methods instead of implementing `std::str::FromStr`, preventing use with `.parse()`.

**Recommendation:** Implement the `FromStr` trait instead.

### L3. Duplicate Disease Rule Logic

**Files:** `disease_pressure.rs`, `fungicide.rs`, `gray_leaf_spot.rs`, `pythium_blight.rs`, `red_thread.rs`

FRAC rotation guidance, nitrogen deficiency checks, and data point construction are duplicated across 5 rule files.

**Recommendation:** Extract shared disease-rule logic into a common module.

### L4. Recommendation State Lost on Restart

**File:** `backend/src/state.rs:15`

`recommendation_states` is an in-memory `HashMap`. Any restart loses user-dismissed/addressed states.

**Recommendation:** Persist to database. The `recommendation_states` table could be a simple key-value store with recommendation ID and state.

### L5. `DataSyncService::initialize` Makes Network Calls in Constructor

**File:** `backend/src/logic/data_sync.rs:31-88`

Constructor establishes TCP connections during startup. Slow/failing external services delay server startup.

**Recommendation:** Use lazy initialization or background connection establishment.

### L6. No Lazy Loading for Route Components

**File:** `frontend/src/App.tsx:3-9`

All 6 pages are eagerly imported into a single bundle.

**Recommendation:** Use `React.lazy()` + `Suspense` for route-based code splitting.

### L7. Hardcoded Data Source Labels in Rules

**Scope:** All 18 rule files

Strings like `"Patio Sensor"`, `"NOAA USCRN"`, `"OpenWeatherMap"` are hardcoded throughout.

**Recommendation:** Source from the `DataSource` enum or configuration.

### L8. Inline Style Objects Recreated Every Render

**Files:** `Dashboard.tsx`, `Applications.tsx`, `Calendar.tsx`, `Recommendations.tsx`

Objects like `{ ...styles.badge, backgroundColor: ... }` are recreated on every render, preventing reference equality optimizations.

**Recommendation:** Use `useMemo` or extract to constants where the values are static.

### L9. `Config` Derives `Serialize` Unnecessarily

**File:** `backend/src/config.rs:4`

Config is never serialized. The derive exposes secret fields to any serialization path.

**Recommendation:** Remove `Serialize` derive from `Config` and sub-structs.

### L10. Error Messages from Server Displayed Raw in UI

**File:** `frontend/src/api/client.ts:29-30`, displayed in `Dashboard.tsx:44`, `Applications.tsx:75`, `Environmental.tsx:74`

If the server returns stack traces or internal paths in error responses, they are shown directly to the user.

**Recommendation:** Sanitize or truncate error messages before displaying.

### L11. No Docker Build Caching in CI

**File:** `.github/workflows/docker-publish.yml`

Each CI build starts from scratch with no cache.

**Recommendation:** Add `cache-from: type=gha` / `cache-to: type=gha,mode=max` to the build action.

### L12. PostgreSQL Port Exposed to Host

**File:** `docker-compose.yml:9-10`

Port `5433:5432` mapping is only needed for development.

**Recommendation:** Move to a `docker-compose.override.yml` for development, remove from production config.

### L13. No Container Security Scanning in CI

**Recommendation:** Add Trivy or Grype scanning to catch CVEs in base images and dependencies.

### L14. Unsafe Type Assertions in Settings Form

**File:** `frontend/src/pages/Settings.tsx:86-90`

`as GrassType`, `as SoilType`, `as IrrigationType` casts without validation. Works by coincidence (falsy empty string falls through to `undefined`) but is brittle.

**Recommendation:** Add explicit validation before casting.

---

## 5. Summary

| Priority | Count | Categories |
|----------|-------|------------|
| **Critical** | 4 | Server panics from `.unwrap()` (will crash on unexpected data) |
| **High** | 8 | Security (CORS, no auth, credential exposure), CI gaps, accessibility |
| **Medium** | 13 | Performance (unbounded queries, mutex contention), UX (polling, double-click), infrastructure |
| **Low** | 14 | Code quality (duplication, magic numbers, conventions), minor optimizations |

### Recommended Action Plan

**Phase 1 — Stability (Critical + High Security)**
- Fix all `.unwrap()` panics (C1-C4)
- Add custom `Debug` impls to redact passwords (H3)
- Restrict CORS (H1)
- Add CI test/lint jobs (H5)
- Create `.env.example` (H6)

**Phase 2 — Performance + UX**
- Fix Mutex contention (H4)
- Add date-range filtering and pagination (M2, M3)
- Add environmental_cache retention policy (M1)
- Fix migration index bug (M4)
- Pause polling in background tabs (M5)
- Add double-click guards (M7)

**Phase 3 — Accessibility**
- Fix `aria-current` regression (H7)
- Add ARIA roles to Gauge (H7)
- Make interactive elements keyboard-accessible (H7)
- Associate form labels (H7)

**Phase 4 — Code Quality**
- Extract shared styles and gauge configs (M8, M9)
- Centralize agronomic thresholds as constants (L1)
- Deduplicate disease rule logic (L3)
- Persist recommendation state (L4)

### What's Working Well

- **Docker setup is solid**: multi-stage build, non-root user, health checks, pinned image digests
- **Agronomic rules are well-designed**: pure functions with no IO, clean separation of concerns
- **API client has good defaults**: timeout via AbortController, proper error propagation
- **Database migrations are well-structured**: proper use of `IF NOT EXISTS`, constraints, and indexes
- **Release profile is optimized**: LTO, strip, single codegen unit for minimal binary size
- **Security scanning (ggshield) is in place** for secrets detection
