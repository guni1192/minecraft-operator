ARG RUST_VERSION=1.63

FROM rust:${RUST_VERSION}-slim-buster as builder

RUN apt-get update \
    && apt-get install -y libssl-dev pkg-config \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

RUN cargo init /minecraft-operator
WORKDIR /minecraft-operator
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/minecraft-operator/target \
    cargo build --release

COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock
COPY src/* src/

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/minecraft-operator/target \
    cargo build --release \
    && mv target/release/minecraft-operator /tmp/minecraft-operator

FROM gcr.io/distroless/cc

# COPY --chown=nonroot:nonroot \
COPY --from=builder \
    /tmp/minecraft-operator /
# USER nonroot

ENTRYPOINT ["/minecraft-operator"]
