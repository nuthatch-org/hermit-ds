# Multi-stage build for the Hermit Data Service indexer + gateway.
FROM rust:slim-bookworm AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev ca-certificates git && rm -rf /var/lib/apt/lists/*
COPY . .
RUN cargo build --release --bin hermit-indexer --bin hermit-gateway

FROM debian:bookworm-slim AS indexer
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/hermit-indexer /usr/local/bin/hermit-indexer
ENTRYPOINT ["hermit-indexer"]

FROM debian:bookworm-slim AS gateway
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/hermit-gateway /usr/local/bin/hermit-gateway
ENV GATEWAY_CONFIG=/app/config.toml
EXPOSE 8090
ENTRYPOINT ["hermit-gateway"]
