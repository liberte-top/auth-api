# Build stage
FROM rust:1.91-bookworm AS builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo fetch
RUN cargo build --release --locked

# Runtime stage (distroless for smaller footprint)
FROM gcr.io/distroless/cc-debian12
WORKDIR /app
COPY --from=builder /app/target/release/auth-api /app/auth-api

USER nonroot:nonroot
EXPOSE 3333
ENTRYPOINT ["/app/auth-api"]
