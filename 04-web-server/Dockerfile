# syntax=docker/dockerfile:1

# --- Builder Stage ---
FROM rust:1.80-slim AS builder
WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Cache deps: copy manifest first
COPY Cargo.toml ./Cargo.toml
# Create empty src to build dependencies if needed
RUN mkdir -p src && echo "fn main(){}" > src/main.rs
RUN cargo build --release || true

# Now copy real sources and migrations
RUN rm -rf src
COPY src ./src
COPY migrations ./migrations

# Set SQLX offline to avoid DB at build time (optional)
ENV SQLX_OFFLINE=true

# Build release binary for this crate
RUN cargo build --bin 04-web-server --release

# --- Runtime Stage ---
FROM debian:bookworm-slim AS runtime
WORKDIR /app

# Create non-root user
RUN useradd -ms /bin/bash appuser

# Copy binary
COPY --from=builder /app/target/release/04-web-server /app/04-web-server

# Expose port
EXPOSE 8080

# Drop privileges
USER appuser

ENV RUST_LOG=info
CMD ["/app/04-web-server"]
