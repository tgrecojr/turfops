# @spec DEPLOY-IMAGE-001
# Stage 1: Build frontend
FROM node:26-alpine@sha256:40cd011b55415bc894e45b02fdd5a5dfde056921b7d23a64c74f80497af48797 AS frontend-build
WORKDIR /app/frontend
COPY frontend/package*.json ./
# @spec DEPLOY-IMAGE-010
# No package lifecycle scripts, as in the supply-chain scan: nothing in the build needs one.
RUN npm ci --ignore-scripts
COPY frontend/ ./
RUN npm run build

# Stage 2: Build backend
FROM rust:1.99-slim-trixie@sha256:aab42391a986424e22d6a3ec14f37bc6dd3dacff4d457ca7cccd781e02297cc6 AS backend-build
WORKDIR /app

# g++ is required to compile DuckDB's bundled C++ amalgamation (duckdb crate, `bundled`).
# tzdata is only here to be copied into the runtime stage (see below).
RUN apt-get update \
    && apt-get install -y --no-install-recommends g++ tzdata \
    && rm -rf /var/lib/apt/lists/*

# @spec DEPLOY-IMAGE-003
# Statically link the C++ runtime (libstdc++) and libgcc into the binary so it only
# depends on glibc at runtime — the cgr.dev/chainguard/glibc-dynamic image provides
# glibc but NOT libstdc++.so.6, which the bundled DuckDB would otherwise need.
ENV RUSTFLAGS="-C link-arg=-static-libstdc++ -C link-arg=-static-libgcc"

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
# @spec DEPLOY-IMAGE-002, DEPLOY-IMAGE-005, DEPLOY-IMAGE-006
# @spec DEPLOY-TZ-002 (the image sets no TZ; the deployment supplies the lawn's zone)
FROM cgr.dev/chainguard/glibc-dynamic:latest@sha256:2827063c9efb4ba971d8621eb75f4e61b8d19850a6293a797df37358cc6f687b

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
