FROM rust:1.50-buster as builder

RUN cargo install cargo-watch

WORKDIR /app/

COPY ./ /app/

RUN cargo build --release --package spacetraders-daemon --bin spacetradersd

ENTRYPOINT ["/app/entrypoint.sh"]

FROM debian:buster

RUN mkdir -p /app \
    && useradd -ms /bin/bash spacetraders \
    && chown -R spacetraders:spacetraders /app \
    && apt-get update \
    && apt-get install -y pkg-config libssl-dev ca-certificates sqlite3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

USER spacetraders

COPY --from=builder --chown=spacetraders:spacetraders /app/target/release/spacetradersd /app/
COPY --chown=spacetraders:spacetraders ./docker/daemon/entrypoint.sh /app/

ENTRYPOINT ["/app/entrypoint.sh"]
