# Stage 1: Build frontend
FROM node:26-alpine@sha256:0b36e8c136b94cd4fcf02188228e76c31ad5872eef3fec8cbd2eee500cfd9e80 AS frontend-build
WORKDIR /app/frontend
COPY frontend/package*.json ./
RUN npm ci
COPY frontend/ ./
RUN npm run build

# Stage 2: Build backend
FROM rust:1.98-slim-trixie@sha256:4cd829461bd5c4d511c32e269da9cb8929223b666519d8004e35fc8d1d771ab7 AS backend-build
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
# Docs the MCP server embeds as resources (include_str! of ../../../docs from src/mcp/).
COPY docs/timing-windows.md docs/agronomy-methods.md /docs/
RUN touch src/main.rs

# Build the application
RUN cargo build --release

# Stage 3: Runtime
FROM cgr.dev/chainguard/glibc-dynamic:latest@sha256:6acf5a19a988abdaf0f3d30247561431a206034e702871442bed66a2c68cc1a2

WORKDIR /app

COPY --from=backend-build --chown=65532:65532 /app/target/release/turfops-backend ./turfops-backend
COPY --from=frontend-build --chown=65532:65532 /app/frontend/dist ./static
# The distroless runtime has no timezone database; without it chrono's `Local` silently
# stays on UTC whatever TZ says, and every date-gated rule flips a day early each evening.
COPY --from=backend-build /usr/share/zoneinfo /usr/share/zoneinfo

ENV STATIC_DIR=/app/static

EXPOSE 3000

ENTRYPOINT ["./turfops-backend"]
