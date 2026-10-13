# ── Stage 1: Build Web Frontend (React + Vite PWA) ──
FROM node:22-slim AS web-builder
WORKDIR /app/web

# Install dependencies
COPY web/package.json web/package-lock.json ./
RUN npm ci

# Copy sources and OpenAPI schema
COPY web/ ./
RUN npm run build

# ── Stage 2: Build Rust Binaries (Daemon + CLI) ──
FROM rust:1.85-slim-bookworm AS rust-builder
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy cargo manifests
COPY Cargo.toml Cargo.lock ./
COPY crates/let-timer-core/Cargo.toml crates/let-timer-core/
COPY crates/let-timer-cli/Cargo.toml crates/let-timer-cli/
COPY crates/let-timer-daemon/Cargo.toml crates/let-timer-daemon/
COPY crates/let-timer-http/Cargo.toml crates/let-timer-http/

# Copy all source crates
COPY crates/ ./crates/

# Copy built web dist for rust-embed
COPY --from=web-builder /app/web/dist ./web/dist

# Build release binaries with embedded web UI
RUN cargo build --release --bin let-timer-daemon --bin let-timer --features let-timer-daemon/embed-web

# ── Stage 3: Minimal Production Runtime ──
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    tzdata \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user and persistent /data directory
RUN useradd -u 1000 -U -s /bin/sh -d /data let-timer \
    && mkdir -p /data \
    && chown -R let-timer:let-timer /data

WORKDIR /data

# Copy compiled binaries from builder stage
COPY --from=rust-builder /app/target/release/let-timer-daemon /usr/local/bin/let-timer-daemon
COPY --from=rust-builder /app/target/release/let-timer /usr/local/bin/let-timer

# Switch to non-root user
USER let-timer:let-timer

ENV LET_TIMER_DB=/data/let-timer.db
ENV LET_TIMER_HTTP=0.0.0.0:8765
ENV RUST_LOG=info

EXPOSE 8765
VOLUME ["/data"]

HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
  CMD curl -f http://127.0.0.1:8765/api/v1/health || exit 1

STOPSIGNAL SIGTERM

ENTRYPOINT ["/usr/local/bin/let-timer-daemon"]
CMD ["--http", "0.0.0.0:8765"]
