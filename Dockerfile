# Stage 1: Build frontend
FROM node:26-alpine@sha256:2d984a15c9b54fd0aeb608b8e0d0d83529eb34d2966db27a1fb4f1edc3d298a3 AS frontend-build
WORKDIR /app/frontend
COPY frontend/package*.json ./
RUN npm ci
COPY frontend/ ./
RUN npm run build

# Stage 2: Build backend
FROM rust:1.98-slim-trixie@sha256:bce1476d4be4d78b83705bc5f428b86d640eeeea33e9dadafbc037b5703a53bf AS backend-build
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
FROM cgr.dev/chainguard/glibc-dynamic:latest@sha256:94ec8c23c45c7aad22b6ab400dc7e1b46dd36f4c71d6c7a3976c8ad4e36ca266

WORKDIR /app

COPY --from=backend-build --chown=65532:65532 /app/target/release/turfops-backend ./turfops-backend
COPY --from=frontend-build --chown=65532:65532 /app/frontend/dist ./static

ENV STATIC_DIR=/app/static

EXPOSE 3000

ENTRYPOINT ["./turfops-backend"]
