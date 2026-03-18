# ═══════════════════════════════════════════════════════════════
# Prometheus — Multi-stage Docker build
#
# Stages:
#   1. chef       — cargo-chef for dependency caching
#   2. planner    — generate dependency recipe
#   3. wasm       — compile Leptos UI to WebAssembly
#   4. tailwind   — generate optimized CSS
#   5. builder    — compile Rust server binary
#   6. runtime    — minimal production image
# ═══════════════════════════════════════════════════════════════

# ── Stage 1: cargo-chef base ──────────────────────────────────
FROM rust:1.75-bookworm AS chef
RUN cargo install cargo-chef
WORKDIR /app

# ── Stage 2: Dependency planner ───────────────────────────────
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ── Stage 3: WASM build (Leptos UI) ──────────────────────────
FROM rust:1.75-bookworm AS wasm
RUN rustup target add wasm32-unknown-unknown && \
    cargo install trunk wasm-bindgen-cli

WORKDIR /app

# Copy workspace files needed for WASM build
COPY Cargo.toml Cargo.lock ./
COPY crates/prometheus-ui/ crates/prometheus-ui/

# Build WASM output
RUN cd crates/prometheus-ui && \
    trunk build --release --dist /app/dist/wasm

# ── Stage 4: Tailwind CSS ────────────────────────────────────
FROM node:20-slim AS tailwind
WORKDIR /app

# Install Tailwind CLI
RUN npm install -g tailwindcss

COPY tailwind.config.js input.css ./
COPY crates/prometheus-ui/src/ crates/prometheus-ui/src/
COPY crates/prometheus-server/src/ crates/prometheus-server/src/

RUN npx tailwindcss -i input.css -o /app/dist/css/prometheus.css --minify

# ── Stage 5: Rust server build ───────────────────────────────
FROM chef AS builder

# Cook dependencies first (cached layer)
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Copy full source
COPY . .

# Copy WASM artifacts into the static directory
COPY --from=wasm /app/dist/wasm/ static/wasm/

# Copy Tailwind CSS output
COPY --from=tailwind /app/dist/css/ static/css/

# Build the server binary
RUN cargo build --release --bin prometheus-server

# Also cross-compile the edge daemon for ARM (optional, if cross is available)
# RUN rustup target add armv7-unknown-linux-musleabihf && \
#     cargo build --release --target armv7-unknown-linux-musleabihf --bin prometheus-edge

# ── Stage 6: Runtime ─────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        ca-certificates \
        libssl3 \
        chromium \
        fonts-inter \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN groupadd --gid 1000 prometheus && \
    useradd --uid 1000 --gid prometheus --shell /bin/bash --create-home prometheus

WORKDIR /app

# Copy the compiled binary
COPY --from=builder /app/target/release/prometheus-server /app/prometheus-server

# Copy static assets (WASM + CSS + other statics)
COPY --from=builder /app/static/ /app/static/
COPY assets/ /app/assets/

# Copy Gradient agent source (for reference / sidecar deployment)
COPY crates/prometheus-agent/ /app/agent/

# Set environment defaults
ENV PROMETHEUS_PORT=3030 \
    PROMETHEUS_HOST=0.0.0.0 \
    AEGIS_DB_URL=http://aegis-db:9091 \
    PUPPETEER_EXECUTABLE_PATH=/usr/bin/chromium \
    RUST_LOG=prometheus_server=info,tower_http=info

EXPOSE 3030

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:3030/health || exit 1

# Run as non-root
USER prometheus

ENTRYPOINT ["/app/prometheus-server"]
CMD ["--port", "3030"]
