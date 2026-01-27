# Build stage
FROM rust:1.78 AS builder
WORKDIR /app

# Copy manifest and sources (required for target discovery)
COPY Cargo.toml Cargo.lock* ./
COPY src ./src

# Build
RUN cargo fetch
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/auth-api /app/auth-api

# No EXPOSE: do not publish ports by default
CMD ["/app/auth-api"]
