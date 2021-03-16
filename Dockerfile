FROM rust:1.50-buster as builder

RUN mkdir -p /app/spacetraders

RUN cargo install cargo-watch

WORKDIR /app/

COPY ./ /app/

RUN cargo build --release

ENTRYPOINT ["/app/entrypoint.sh"]

FROM debian:buster

RUN mkdir /app \
    && useradd -ms /bin/bash spacetraders \
    && chown -R spacetraders:spacetraders /app \
    && apt-get update \
    && apt-get install -y pkg-config libssl-dev ca-certificates sqlite3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

USER spacetraders

COPY --from=builder --chown=spacetraders:spacetraders /app/target/release/spacetraders-rs /app/
COPY --chown=spacetraders:spacetraders ./entrypoint.sh /app/

ENTRYPOINT ["/app/entrypoint.sh"]
