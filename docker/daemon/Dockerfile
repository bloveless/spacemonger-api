FROM rust:1.51-buster as planner

WORKDIR app

RUN cargo install cargo-chef --version 0.1.20

COPY . .

RUN cargo chef prepare --recipe-path recipe.json

FROM rust:1.51-buster as cacher

WORKDIR app
RUN cargo install cargo-chef --version 0.1.20
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

FROM rust:1.51-buster as builder

WORKDIR /app/

COPY . .

COPY --from=cacher /app/target target
COPY --from=cacher /usr/local/cargo /usr/local/cargo

RUN cargo build --release --package spacemonger-daemon --bin spacemongerd

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

COPY --from=builder --chown=spacemonger:spacemonger /app/target/release/spacemongerd /app/
COPY --chown=spacemonger:spacemonger ./docker/daemon/entrypoint.sh /app/

ENTRYPOINT ["/app/entrypoint.sh"]
