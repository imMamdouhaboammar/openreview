# Multi-stage Dockerfile for OpenReview-RS
FROM rust:1-slim AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN apt-get update && apt-get install -y --no-install-recommends pkg-config libssl-dev cmake && rm -rf /var/lib/apt/lists/*
RUN cargo build --release

# Runtime Stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    git \
    ca-certificates \
    curl \
    bash \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/openreview /usr/local/bin/openreview
COPY .agents /app/.agents

ENV HOST=0.0.0.0
ENV PORT=3000
ENV SANDBOX_DRIVER=local

EXPOSE 3000

ENTRYPOINT ["openreview", "serve"]
