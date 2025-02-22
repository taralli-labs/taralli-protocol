# Why bookworm? See: https://community.fly.io/t/rust-server-missing-libssl-so-3-on-new-deploy/15114/4
FROM lukemathwalker/cargo-chef:latest-rust-bookworm AS chef
WORKDIR /app

ARG CARGO_PROFILE_TEST_DEBUG=0

FROM chef AS planner
RUN apt-get update && apt-get install -y --no-install-recommends \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --locked --features ci-test --package taralli-server

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends \
    libssl3 ca-certificates \
    && update-ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY example_server_config.json /app/config.json
COPY --from=builder /app/target/release/server /usr/bin
EXPOSE 8000
ENTRYPOINT ["server"]