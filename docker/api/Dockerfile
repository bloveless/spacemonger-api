FROM rust:1.50-buster as builder

RUN cargo install cargo-watch

WORKDIR /app/

COPY ./ /app/

RUN cargo build --release --package spacemonger-api --bin spacemonger-api

ENTRYPOINT ["/app/entrypoint.sh"]

FROM debian:buster

RUN mkdir -p /app \
    && useradd -ms /bin/bash spacemonger \
    && chown -R spacemonger:spacemonger /app \
    && apt-get update \
    && apt-get install -y pkg-config libssl-dev ca-certificates sqlite3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

USER spacemonger

COPY --from=builder --chown=spacemonger:spacemonger /app/target/release/spacemonger-api /app/
COPY --chown=spacemonger:spacemonger ./docker/api/entrypoint.sh /app/

ENTRYPOINT ["/app/entrypoint.sh"]
