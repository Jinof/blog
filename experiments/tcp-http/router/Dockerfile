FROM rust:1.93-bookworm AS builder

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ethtool iproute2 iputils-ping \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/tcp-http-lab-router /usr/local/bin/lab-router
COPY entrypoint.sh /usr/local/bin/router-entrypoint

ENTRYPOINT ["/usr/local/bin/router-entrypoint"]
