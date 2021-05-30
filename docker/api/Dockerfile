FROM rust:1.52.1-buster as planner-api

WORKDIR app

RUN cargo install cargo-chef --version 0.1.20

COPY . .

RUN cargo chef prepare --recipe-path recipe.json

FROM rust:1.52.1-buster as cacher-api

WORKDIR app
RUN cargo install cargo-chef --version 0.1.20
COPY --from=planner-api /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

FROM rust:1.52.1-buster as builder-api

WORKDIR /app/

COPY . .

COPY --from=cacher-api /app/target target
COPY --from=cacher-api /usr/local/cargo /usr/local/cargo

RUN cargo build --release --package spacemonger-api --bin spacemonger-api

ENTRYPOINT ["/app/entrypoint.sh"]

FROM debian:buster-slim

RUN mkdir -p /app \
    && useradd -ms /bin/bash spacemonger \
    && chown -R spacemonger:spacemonger /app \
    && apt-get update \
    && apt-get install -y libssl-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

USER spacemonger

COPY --from=builder-api --chown=spacemonger:spacemonger /app/target/release/spacemonger-api /app/
COPY --chown=spacemonger:spacemonger ./docker/api/entrypoint.sh /app/

ENTRYPOINT ["/app/entrypoint.sh"]
