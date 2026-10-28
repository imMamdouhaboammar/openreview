# Multi-stage Dockerfile for OpenReview
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
    docker.io \
    && rm -rf /var/lib/apt/lists/*

# Non-root application user for defense-in-depth isolation
RUN groupadd -g 10001 appgroup && \
    useradd -u 10001 -g appgroup -s /bin/bash -m appuser

WORKDIR /app
COPY --from=builder /app/target/release/openreview /usr/local/bin/openreview
COPY .agents /app/.agents

# Create persistent state directory with appropriate non-root ownership
RUN mkdir -p /data /tmp/openreview /srv/openreview/workspaces && \
    chown -R appuser:appgroup /app /data /tmp/openreview /srv/openreview/workspaces

USER appuser:appgroup

ENV HOST=0.0.0.0
ENV PORT=3000
ENV SANDBOX_DRIVER=docker

EXPOSE 3000

HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
    CMD curl -f "http://localhost:${PORT:-3000}/health" || exit 1

ENTRYPOINT ["openreview"]
CMD ["serve"]
