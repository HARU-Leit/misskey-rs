# Build stage
FROM rust:1.85-bookworm AS builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests first for better layer caching
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Build release binary
# Note: package is misskey-server but binary name is 'misskey' (defined in [[bin]])
RUN cargo build --release --package misskey-server

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder (binary name is 'misskey', not 'misskey-server')
COPY --from=builder /app/target/release/misskey /app/misskey

# Copy default config
COPY config /app/config

# Create non-root user
RUN useradd -r -s /bin/false misskey && \
    chown -R misskey:misskey /app
USER misskey

# Expose port
EXPOSE 3000

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=10s --retries=3 \
    CMD curl -sf -X POST http://localhost:3000/api/meta || exit 1

# Run
CMD ["./misskey"]
