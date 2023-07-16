ARG RUST_VERSION=1.68

FROM rust:${RUST_VERSION}-slim-bullseye as builder

RUN apt-get update && \
    apt-get install -y libssl-dev pkg-config && \
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

LABEL org.opencontainers.image.authors="Takashi Iiguni <iiguni.tks@gmail.com>"
LABEL org.opencontainers.image.url="https://github.com/guni1192/minecraft-operator"
LABEL org.opencontainers.image.source="https://github.com/guni1192/minecraft-operator/blob/main/Dockerfile"
# LABEL org.opencontainers.image.version="0.1.0"

COPY --from=builder \
    /tmp/minecraft-operator /

ENTRYPOINT ["/minecraft-operator"]
