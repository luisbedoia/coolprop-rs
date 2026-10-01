# Build/test image: Emscripten (CoolProp C++ -> wasm) + Rust.
# Keep RUST_VERSION in sync with rust-toolchain.toml.
FROM emscripten/emsdk:6.0.10

ARG RUST_VERSION=1.99.0

RUN apt-get update && apt-get install -y --no-install-recommends \
      build-essential cmake git ca-certificates curl python3 pkg-config \
      libclang-dev clang \
    && rm -rf /var/lib/apt/lists/* \
    # The repo (and CoolProp's CPM cache) is bind-mounted with the host's UID.
    && git config --system --add safe.directory '*'

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
      | sh -s -- -y --default-toolchain "$RUST_VERSION" --profile minimal \
        --target wasm32-unknown-emscripten \
        --component rustfmt --component clippy \
    && chmod -R a+w "$CARGO_HOME" "$RUSTUP_HOME"

WORKDIR /work
