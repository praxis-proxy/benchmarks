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
COPY crates/benchmarks-probe/Cargo.toml crates/benchmarks-probe/Cargo.toml

RUN mkdir -p crates/benchmarks-probe/src \
    && echo '//! stub' > crates/benchmarks-probe/src/lib.rs \
    && printf '//! stub\nfn main() {}\n' > crates/benchmarks-probe/src/main.rs

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release -p benchmarks-probe

# ------------------------------------------------------------------------------
# Cache Tricks
# ------------------------------------------------------------------------------

# Replace stubs with real source, then rebuild. Only the
# project crates recompile; all dependencies are cached.

COPY crates/benchmarks-probe/src crates/benchmarks-probe/src

# Touch the source files so cargo sees them as newer than
# the cached stub artifacts.
RUN find crates -name '*.rs' -exec touch {} +

# ------------------------------------------------------------------------------
# Build
# ------------------------------------------------------------------------------

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release -p benchmarks-probe \
    && cp target/release/benchmarks-probe /usr/local/bin/benchmarks-probe

# ------------------------------------------------------------------------------
# Stage 2: Runtime
# ------------------------------------------------------------------------------

FROM alpine:3.23

LABEL org.opencontainers.image.source="https://github.com/praxis-proxy/benchmarks" \
    org.opencontainers.image.description="Praxis benchmarks probe binary" \
    org.opencontainers.image.licenses="Apache-2.0"

RUN apk add --no-cache ca-certificates \
    && addgroup -S probe \
    && adduser -S -G probe -h /nonexistent -s /sbin/nologin probe

COPY --from=builder --chown=root:root --chmod=0555 \
    /usr/local/bin/benchmarks-probe /usr/local/bin/benchmarks-probe

USER probe:probe

# When scaffolding a long-running service, add EXPOSE and a HEALTHCHECK
# here and update the container workflow to wait for healthy status.

ENTRYPOINT ["benchmarks-probe"]
