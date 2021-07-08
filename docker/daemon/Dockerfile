FROM golang:1.16.5-buster as builder

WORKDIR /app

COPY go.mod .
COPY go.sum .

RUN go mod download

COPY . .

RUN go build -o ./out/spacemongerd ./cmd/daemon/*.go

# ----------------------------------------------- #

FROM debian:buster

RUN mkdir -p /app \
    && useradd -ms /bin/bash spacemonger \
    && chown -R spacemonger:spacemonger /app \
    && apt-get update \
    && apt-get install -y libssl-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY ./.env.prod ./.env
COPY ./migrations ./migrations

COPY --from=builder /app/out/spacemongerd /app/spacemongerd
RUN chown -R spacemonger:spacemonger /app/spacemongerd

USER spacemonger

CMD ["/app/spacemongerd"]
