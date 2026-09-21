# Stage 1: Build frontend
FROM node:26-alpine@sha256:dbaa92e5758cbbcf85d65d5403fdb530fe3442cbe8c6dbfb7ef23365450d5070 AS frontend-build
WORKDIR /app/frontend
COPY frontend/package*.json ./
RUN npm ci
COPY frontend/ ./
RUN npm run build

# Stage 2: Build backend
FROM rust:1.98-slim-trixie@sha256:f47a8de237dcbb0b0ce1099901e60a89728e3d51f24e664b40e947171538ade7 AS backend-build
WORKDIR /app

# g++ is required to compile DuckDB's bundled C++ amalgamation (duckdb crate, `bundled`).
# tzdata is only here to be copied into the runtime stage (see below).
RUN apt-get update \
    && apt-get install -y --no-install-recommends g++ tzdata \
    && rm -rf /var/lib/apt/lists/*

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
RUN touch src/main.rs

# Build the application
RUN cargo build --release

# Stage 3: Runtime
FROM cgr.dev/chainguard/glibc-dynamic:latest@sha256:94ec8c23c45c7aad22b6ab400dc7e1b46dd36f4c71d6c7a3976c8ad4e36ca266

WORKDIR /app

COPY --from=backend-build --chown=65532:65532 /app/target/release/turfops-backend ./turfops-backend
COPY --from=frontend-build --chown=65532:65532 /app/frontend/dist ./static
# The distroless runtime has no timezone database; without it chrono's `Local` silently
# stays on UTC whatever TZ says, and every date-gated rule flips a day early each evening.
COPY --from=backend-build /usr/share/zoneinfo /usr/share/zoneinfo

ENV STATIC_DIR=/app/static

EXPOSE 3000

ENTRYPOINT ["./turfops-backend"]
