# syntax=docker/dockerfile:1.7
# artcraftcloud.com: one image, one process, no volume (nothing is stored).
# Build:  docker build -t artcraft-cloud-web .
# Run:    docker run -d -p 8080:8080 artcraft-cloud-web

ARG RUST_IMAGE=docker.io/library/rust:1-slim-bookworm
ARG RUNTIME_IMAGE=gcr.io/distroless/cc-debian12:nonroot

FROM ${RUST_IMAGE} AS build
ARG TRUNK_VERSION=0.22.2
ARG TRUNK_SHA256=4a90216d93c553f3d8b7401fce0dee588e0fb1eb1c6a379ebac81fabe820f25b
# Must match wasm-bindgen in Cargo.lock and [tools] in crates/wizard/Trunk.toml.
ARG WASM_BINDGEN_VERSION=0.2.130
ARG WASM_BINDGEN_SHA256=9687f3bb551b9f66f37ed32a40d7913a00f7962db653054e56857bc306c3227f
ARG BINARYEN_VERSION=131
ARG BINARYEN_SHA256=b5bf1f0eaf17c63ee588ff7a5954dc8f6ce2c26989051c66f24dfe9ece3e46db
RUN apt-get update && apt-get install -y --no-install-recommends curl ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && rustup target add wasm32-unknown-unknown \
    && fetch() { curl -fsSL "$1" -o /tmp/dl && echo "$2  /tmp/dl" | sha256sum -c - >/dev/null; } \
    && fetch "https://github.com/trunk-rs/trunk/releases/download/v${TRUNK_VERSION}/trunk-x86_64-unknown-linux-musl.tar.gz" "$TRUNK_SHA256" \
    && tar -xzf /tmp/dl -C /usr/local/bin trunk \
    && fetch "https://github.com/wasm-bindgen/wasm-bindgen/releases/download/${WASM_BINDGEN_VERSION}/wasm-bindgen-${WASM_BINDGEN_VERSION}-x86_64-unknown-linux-musl.tar.gz" "$WASM_BINDGEN_SHA256" \
    && tar -xzf /tmp/dl -C /usr/local/bin --strip-components=1 "wasm-bindgen-${WASM_BINDGEN_VERSION}-x86_64-unknown-linux-musl/wasm-bindgen" \
    && fetch "https://github.com/WebAssembly/binaryen/releases/download/version_${BINARYEN_VERSION}/binaryen-version_${BINARYEN_VERSION}-x86_64-linux.tar.gz" "$BINARYEN_SHA256" \
    && tar -xzf /tmp/dl -C /usr/local/bin --strip-components=2 "binaryen-version_${BINARYEN_VERSION}/bin/wasm-opt" \
    && rm /tmp/dl
WORKDIR /src
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release --locked -p server \
    && cp target/release/artcraft-cloud-web /usr/local/bin/ \
    && cd crates/wizard && trunk build --release --offline

FROM ${RUNTIME_IMAGE}
COPY --from=build /usr/local/bin/artcraft-cloud-web /usr/local/bin/artcraft-cloud-web
COPY --from=build /src/crates/site/static /app/static
COPY --from=build /src/crates/wizard/dist /app/wizard
ENV ACW_LISTEN=0.0.0.0:8080 \
    ACW_STATIC_DIR=/app/static \
    ACW_WIZARD_DIR=/app/wizard \
    TOKIO_WORKER_THREADS=2
EXPOSE 8080
USER nonroot
ENTRYPOINT ["/usr/local/bin/artcraft-cloud-web"]
