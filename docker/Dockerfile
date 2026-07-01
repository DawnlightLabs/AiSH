# syntax=docker/dockerfile:1

FROM debian:bookworm AS llama-builder

ARG LLAMA_CPP_COMMIT=931eb37f8cac5a6ca84d5641445d460af2a9d7dd

RUN apt-get update \
    && apt-get install -y --no-install-recommends build-essential ca-certificates cmake git \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /src/llama.cpp
RUN git init \
    && git remote add origin https://github.com/ggerganov/llama.cpp.git \
    && git fetch --depth 1 origin "${LLAMA_CPP_COMMIT}" \
    && git checkout --detach FETCH_HEAD
RUN cmake -S . -B build \
      -DBUILD_SHARED_LIBS=OFF \
      -DGGML_NATIVE=OFF \
      -DGGML_OPENMP=OFF \
      -DLLAMA_CURL=OFF \
      -DLLAMA_BUILD_TESTS=OFF \
      -DLLAMA_BUILD_EXAMPLES=ON \
    && cmake --build build --config Release --target llama-cli -j2

FROM rust:1-bookworm AS rust-builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        curl \
        libayatana-appindicator3-dev \
        librsvg2-dev \
        libssl-dev \
        libwebkit2gtk-4.1-dev \
        nodejs \
        npm \
        patchelf \
        pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /src
COPY . .
RUN npm ci
RUN cargo test -p aish-safety
RUN cargo build --release -p aish-provider-shell
RUN npm --workspace apps/desktop run tauri -- build --no-bundle

FROM debian:bookworm-slim AS desktop-runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        bash \
        ca-certificates \
        dbus-x11 \
        gosu \
        libayatana-appindicator3-1 \
        librsvg2-2 \
        libwebkit2gtk-4.1-0 \
        novnc \
        openbox \
        websockify \
        x11vnc \
        xvfb \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --shell /bin/bash aish

COPY --from=rust-builder /src/target/release/aish-desktop /usr/local/bin/aish-desktop
COPY --from=llama-builder /src/llama.cpp/build/bin/llama-cli /usr/local/bin/llama-cli
COPY docker/start-desktop.sh /usr/local/bin/start-desktop

RUN chmod +x /usr/local/bin/start-desktop

WORKDIR /home/aish
EXPOSE 6080
ENTRYPOINT ["/usr/local/bin/start-desktop"]

FROM debian:bookworm-slim AS provider-runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends bash ca-certificates git gosu \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --shell /bin/bash aish

COPY --from=rust-builder /src/target/release/aish /usr/local/bin/aish
COPY --from=llama-builder /src/llama.cpp/build/bin/llama-cli /usr/local/bin/llama-cli
COPY docker/start-provider.sh /usr/local/bin/start-provider

RUN chmod +x /usr/local/bin/start-provider

WORKDIR /workspace
ENTRYPOINT ["/usr/local/bin/start-provider"]
