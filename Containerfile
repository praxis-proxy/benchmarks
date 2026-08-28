# syntax=docker/dockerfile:1

# ------------------------------------------------------------------------------
# Stage 1: Build
# ------------------------------------------------------------------------------

FROM rust:1.96-alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /src

# ------------------------------------------------------------------------------
# Cache Build
# ------------------------------------------------------------------------------

# Cache dependency builds: copy only manifests first, then
# create stub source files so `cargo build` resolves and
# compiles all dependencies without the real source code.
# See: https://shaneutt.com/blog/rust-fast-small-docker-image-builds/

COPY Cargo.toml Cargo.lock ./
COPY crates/praxis-bench/Cargo.toml crates/praxis-bench/Cargo.toml

RUN mkdir -p crates/praxis-bench/src \
    && echo '//! stub' > crates/praxis-bench/src/lib.rs \
    && printf '//! stub\nfn main() {}\n' > crates/praxis-bench/src/main.rs

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release -p praxis-bench

# ------------------------------------------------------------------------------
# Cache Tricks
# ------------------------------------------------------------------------------

# Replace stubs with real source, then rebuild. Only the
# project crates recompile; all dependencies are cached.

COPY crates/praxis-bench/src crates/praxis-bench/src

# Touch the source files so cargo sees them as newer than
# the cached stub artifacts.
RUN find crates -name '*.rs' -exec touch {} +

# ------------------------------------------------------------------------------
# Build
# ------------------------------------------------------------------------------

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release -p praxis-bench \
    && cp target/release/praxis-bench /usr/local/bin/praxis-bench

# ------------------------------------------------------------------------------
# Stage 2: Runtime
# ------------------------------------------------------------------------------

FROM alpine:3.23

LABEL org.opencontainers.image.source="https://github.com/praxis-proxy/benchmarks" \
    org.opencontainers.image.description="Generic Praxis proxy benchmark runner" \
    org.opencontainers.image.licenses="Apache-2.0"

# Pinned load generators and the target versions the harness shells out to.
ARG VEGETA_VERSION=12.13.0
ARG FORTIO_VERSION=1.75.1

# The runner orchestrates proxy containers through the host Docker daemon,
# so it needs the docker CLI plus the vegeta/fortio load generators. Mount
# the host socket and share its network at run time:
#
#   docker run --rm --network host \
#     -v /var/run/docker.sock:/var/run/docker.sock \
#     ghcr.io/praxis-proxy/benchmarks --image ghcr.io/praxis-proxy/praxis:latest
RUN set -eux; \
    apk add --no-cache ca-certificates docker-cli curl tar; \
    case "$(uname -m)" in \
      x86_64) GOARCH=amd64; \
        VEGETA_SHA=e8759ce45c14e18374bdccd3ba6068197bc3a9f9b7e484db3837f701b9d12e61; \
        FORTIO_SHA=92da34238dee258191a9dc6691c8bc75305b308951e934e2c3b4e658db0d77d1;; \
      aarch64) GOARCH=arm64; \
        VEGETA_SHA=950381173a5575e25e8e086f36fc03bf65d61a2433329b48e41e1cb5e4133bba; \
        FORTIO_SHA=f66275a56ef41e9a5afb2ea8181eb53ca36b34c6d19a201b58aec17dbe95a853;; \
      *) echo "unsupported architecture: $(uname -m)" >&2; exit 1;; \
    esac; \
    curl -sSfL -o /tmp/vegeta.tar.gz \
      "https://github.com/tsenart/vegeta/releases/download/v${VEGETA_VERSION}/vegeta_${VEGETA_VERSION}_linux_${GOARCH}.tar.gz"; \
    echo "${VEGETA_SHA}  /tmp/vegeta.tar.gz" | sha256sum -c; \
    tar xz -C /usr/local/bin -f /tmp/vegeta.tar.gz vegeta; \
    curl -sSfL -o /tmp/fortio.tgz \
      "https://github.com/fortio/fortio/releases/download/v${FORTIO_VERSION}/fortio-linux_${GOARCH}-${FORTIO_VERSION}.tgz"; \
    echo "${FORTIO_SHA}  /tmp/fortio.tgz" | sha256sum -c; \
    tar xz -C /usr/local/bin -f /tmp/fortio.tgz usr/bin/fortio --strip-components=2; \
    rm -f /tmp/vegeta.tar.gz /tmp/fortio.tgz

# Bundle the built-in comparison configs and point the resolver at them so
# the runner works without the source tree mounted.
COPY crates/praxis-bench/comparison /opt/praxis-bench/comparison
ENV PRAXIS_BENCH_CONFIG_DIR=/opt/praxis-bench/comparison/configs

COPY --from=builder --chown=root:root --chmod=0555 \
    /usr/local/bin/praxis-bench /usr/local/bin/praxis-bench

ENTRYPOINT ["praxis-bench"]
CMD ["--help"]
