# Stage 1: Build frontend
FROM node:26-alpine@sha256:aadf416b2cdce311a8811ba3f0608a61b77dbf997500e2eafe781b51f6a0b019 AS frontend-build
WORKDIR /app/frontend
COPY frontend/package*.json ./
RUN npm ci
COPY frontend/ ./
RUN npm run build

# Stage 2: Build backend
FROM rust:1.97-slim-trixie@sha256:3b2879047d42784ca9403ad20c51ed3df361a50f1df96f5777d39b4e33aa65cd AS backend-build
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
FROM cgr.dev/chainguard/glibc-dynamic:latest@sha256:57e5704e70a85b90191182eb6110d1c817df0d8e96035cb041195c5a351f0861

WORKDIR /app

COPY --from=backend-build --chown=65532:65532 /app/target/release/turfops-backend ./turfops-backend
COPY --from=frontend-build --chown=65532:65532 /app/frontend/dist ./static

ENV STATIC_DIR=/app/static

EXPOSE 3000

ENTRYPOINT ["./turfops-backend"]
