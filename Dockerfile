# Stage 1: Build frontend
FROM node:26-alpine@sha256:2d984a15c9b54fd0aeb608b8e0d0d83529eb34d2966db27a1fb4f1edc3d298a3 AS frontend-build
WORKDIR /app/frontend
COPY frontend/package*.json ./
RUN npm ci
COPY frontend/ ./
RUN npm run build

# Stage 2: Build backend
FROM rust:1.98-slim-trixie@sha256:17d1ba895198f9934c6314ec5346a0d5115372f3243390c3d731e242f35c2f27 AS backend-build
WORKDIR /app

# g++ is required to compile DuckDB's bundled C++ amalgamation (duckdb crate, `bundled`).
RUN apt-get update \
    && apt-get install -y --no-install-recommends g++ \
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
FROM cgr.dev/chainguard/glibc-dynamic:latest@sha256:d0046044cd28948d3380eb0d98709dc7e63f98161fe7105135e1025650bad17a

WORKDIR /app

COPY --from=backend-build --chown=65532:65532 /app/target/release/turfops-backend ./turfops-backend
COPY --from=frontend-build --chown=65532:65532 /app/frontend/dist ./static

ENV STATIC_DIR=/app/static

EXPOSE 3000

ENTRYPOINT ["./turfops-backend"]
