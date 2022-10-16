ARG RUST_VERSION=1.63

FROM rust:${RUST_VERSION}-slim-bullseye as builder

RUN apt-get update && \
    apt-get install -y pkg-config curl make musl-tools && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

ENV OPENSSL_VERSION=3.0.5
RUN mkdir -p /usr/local/musl/include && \
    ln -s /usr/include/linux /usr/local/musl/include/linux && \
    ln -s /usr/include/x86_64-linux-gnu/asm /usr/local/musl/include/asm && \
    ln -s /usr/include/asm-generic /usr/local/musl/include/asm-generic && \
    cd /tmp && \
    curl -fLO "https://github.com/openssl/openssl/archive/refs/tags/openssl-${OPENSSL_VERSION}.tar.gz" && \
    tar xzf "openssl-${OPENSSL_VERSION}.tar.gz" && \
    cd "openssl-openssl-${OPENSSL_VERSION}" && \
    CC=musl-gcc ./Configure no-shared no-zlib -fPIC --prefix=/usr/local/musl -DOPENSSL_NO_SECURE_MEMORY linux-x86_64 && \
    C_INCLUDE_PATH=/usr/local/musl/include/ make depend && \
    C_INCLUDE_PATH=/usr/local/musl/include/ make && \
    make install && \
    rm /usr/local/musl/include/linux /usr/local/musl/include/asm /usr/local/musl/include/asm-generic && \
    rm -r /tmp/*

ENV X86_64_UNKNOWN_LINUX_MUSL_OPENSSL_DIR=/usr/local/musl/ \
    X86_64_UNKNOWN_LINUX_MUSL_OPENSSL_STATIC=1 \
    PQ_LIB_STATIC_X86_64_UNKNOWN_LINUX_MUSL=1 \
    PG_CONFIG_X86_64_UNKNOWN_LINUX_GNU=/usr/bin/pg_config \
    PKG_CONFIG_ALLOW_CROSS=true \
    PKG_CONFIG_ALL_STATIC=true \
    TARGET=musl

RUN rustup target add x86_64-unknown-linux-musl

RUN cargo init /minecraft-operator
WORKDIR /minecraft-operator

COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/minecraft-operator/target \
    cargo build --release --target=x86_64-unknown-linux-musl && \
    rm -f target/x86_64-unknown-linux-musl/release/minecraft-operator

COPY src/* src/

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/minecraft-operator/target \
    cargo build --release --target=x86_64-unknown-linux-musl && \
    mv target/x86_64-unknown-linux-musl/release/minecraft-operator /tmp/minecraft-operator

FROM gcr.io/distroless/static-debian11

COPY --from=builder \
    /tmp/minecraft-operator /

ENTRYPOINT ["/minecraft-operator"]
