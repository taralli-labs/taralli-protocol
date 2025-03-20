# Variables that used to swap between different build configurations.
# PROFILE: The cargo profile used to compile the crate.
# FEATURES: The cargo features used to compile the crate.
# CONFIG: The configuration file used by the server.
ARG PROFILE=stripped
ARG FEATURES=default
ARG CONFIG=config.json

# Why bookworm? See: https://community.fly.io/t/rust-server-missing-libssl-so-3-on-new-deploy/15114/4
FROM lukemathwalker/cargo-chef:latest-rust-bookworm AS chef
WORKDIR /app

# Install Mold for faster linking
RUN wget https://github.com/rui314/mold/releases/download/v2.37.1/mold-2.37.1-x86_64-linux.tar.gz
RUN tar -xf mold-2.37.1-x86_64-linux.tar.gz
RUN mv mold-2.37.1-x86_64-linux/bin/mold /bin/mold
ENV MOLD_PATH="/bin/mold"
ENV RUSTFLAGS="-C linker=/bin/mold"

FROM chef AS planner
RUN apt-get update && apt-get install -y --no-install-recommends \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
ARG PROFILE
ARG FEATURES
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --profile $PROFILE --recipe-path recipe.json
COPY . .
RUN cargo build --profile $PROFILE --locked --features $FEATURES --bin server

FROM debian:bookworm-slim AS runtime
ARG CONFIG
ARG PROFILE
RUN apt-get update && apt-get install -y --no-install-recommends \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY ${CONFIG} /app/config.json
COPY --from=builder /app/target/${PROFILE}/server /usr/bin
EXPOSE 8000
ENTRYPOINT ["server"]