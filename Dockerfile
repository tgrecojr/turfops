# @spec DEPLOY-IMAGE-001
# Stage 1: Build frontend
FROM node:26-alpine@sha256:c6ad9e9c1d80f6eb607c26b4b0a132ff7a2344fad1c78ea3baa2a7d4c8de6d94 AS frontend-build
WORKDIR /app/frontend
COPY frontend/package*.json ./
# @spec DEPLOY-IMAGE-010
# No package lifecycle scripts, as in the supply-chain scan: nothing in the build needs one.
RUN npm ci --ignore-scripts
COPY frontend/ ./
RUN npm run build

# Stage 2: Build backend
FROM rust:1.99-slim-trixie@sha256:b8e062b1c3a80183df83d9ba187cadd2b5ab84bc78d453dbfeb2fccd5f982247 AS backend-build
WORKDIR /app

# g++ is required to compile DuckDB's bundled C++ amalgamation (duckdb crate, `bundled`).
# tzdata is only here to be copied into the runtime stage (see below).
RUN apt-get update \
    && apt-get install -y --no-install-recommends g++ tzdata \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests first for dependency caching
COPY backend/Cargo.toml backend/Cargo.lock* ./
RUN mkdir src && echo 'fn main() { println!("placeholder"); }' > src/main.rs
RUN cargo build --release 2>/dev/null || true
RUN rm -rf src

# Copy actual source and force recompile
COPY backend/src ./src
# @spec DEPLOY-IMAGE-004
# Docs the MCP server embeds as resources (include_str! of ../../../docs from src/mcp/).
COPY docs/timing-windows.md docs/agronomy-methods.md /docs/
RUN touch src/main.rs

# Build the application
RUN cargo build --release

# Stage 3: Runtime
# @spec DEPLOY-IMAGE-002, DEPLOY-IMAGE-003, DEPLOY-IMAGE-005, DEPLOY-IMAGE-006
# glibc-dynamic ships glibc, libstdc++.so.6 and libgcc_s.so.1, which the binary loads
# (the bundled DuckDB C++ build links the C++ runtime dynamically); nothing else is copied.
# @spec DEPLOY-TZ-002 (the image sets no TZ; the deployment supplies the lawn's zone)
FROM cgr.dev/chainguard/glibc-dynamic:latest@sha256:709673274b417a48f56374da8a7633ef7b2f5b27e34612ce98a335737318cc94

WORKDIR /app

COPY --from=backend-build --chown=65532:65532 /app/target/release/turfops-backend ./turfops-backend
COPY --from=frontend-build --chown=65532:65532 /app/frontend/dist ./static
# @spec DEPLOY-TZ-001
# The distroless runtime has no timezone database; without it chrono's `Local` silently
# stays on UTC whatever TZ says, and every date-gated rule flips a day early each evening.
COPY --from=backend-build /usr/share/zoneinfo /usr/share/zoneinfo

ENV STATIC_DIR=/app/static

# @spec DEPLOY-IMAGE-009
# Run as the distroless non-root user, stated rather than inherited from the base image.
USER 65532:65532

EXPOSE 3000

# No shell or curl in the runtime: the binary probes its own /api/v1/health. The start
# period covers the startup wait for PostgreSQL (about a minute) plus migrations.
HEALTHCHECK --interval=30s --timeout=5s --start-period=90s --retries=3 \
    CMD ["./turfops-backend", "--healthcheck"]

ENTRYPOINT ["./turfops-backend"]
