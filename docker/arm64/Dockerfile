ARG RUST_VERSION=1.63

FROM rust:${RUST_VERSION}-slim-bullseye as builder

RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

RUN cargo init /minecraft-operator
WORKDIR /minecraft-operator

COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/minecraft-operator/target \
    cargo build --release && \
    rm -f target/release/minecraft-operator

COPY src/* src/

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/minecraft-operator/target \
    cargo build --release && \
    mv target/release/minecraft-operator /tmp/minecraft-operator

FROM gcr.io/distroless/cc-debian11

COPY --from=builder \
    /tmp/minecraft-operator /

ENTRYPOINT ["/minecraft-operator"]
