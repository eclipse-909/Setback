# syntax=docker/dockerfile:1

# Dockerfile.local — local development image for server and api
#
# Compiles inside Docker, tuned for fast rebuilds:
#   - BuildKit cache mounts keep the cargo registry AND the target dir between
#     builds, so only changed crates are recompiled (incremental builds).
#   - The target cache is shared by api and server, so dependencies are
#     compiled once for both.
#   - mold linker and reduced debug info make linking much faster.
#
# Needs BuildKit (default in current Docker). Expect the first build to be slow.

ARG PROFILE=debug
ARG PACKAGE
ARG PORT
ARG RUNTIME_DIR

# =============================================================================
# builder — compiles the selected package
# =============================================================================
FROM rust:bookworm AS builder

ARG PROFILE
ARG PACKAGE
ARG RUNTIME_DIR

RUN apt-get update \
    && apt-get install -y --no-install-recommends mold \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Install the toolchain from rust-toolchain.toml in its own layer, so it is
# only redone when that file changes.
COPY rust-toolchain.toml .
RUN rustup toolchain install

# NOTE: setting RUSTFLAGS here overrides any rustflags in .cargo/config.toml.
ENV RUSTFLAGS="-C link-arg=-fuse-ld=mold" \
    CARGO_PROFILE_DEV_DEBUG=line-tables-only

COPY . .

# The target dir lives in a cache mount, so the binary must be copied out of it
# within this same RUN. Output (binary + optional runtime dir) goes to /out.
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,target=/build/target,sharing=locked \
    set -eux; \
    # cargo calls the debug profile "dev", but its output dir is target/debug
    if [ "$PROFILE" = "debug" ] || [ "$PROFILE" = "dev" ]; then \
        CARGO_PROFILE=dev; OUT_DIR=debug; \
    else \
        CARGO_PROFILE="$PROFILE"; OUT_DIR="$PROFILE"; \
    fi; \
    cargo build --profile "$CARGO_PROFILE" -p "$PACKAGE"; \
    mkdir -p /out; \
    cp "target/${OUT_DIR}/${PACKAGE}" /out/app; \
    if [ -n "$RUNTIME_DIR" ]; then \
        cp -r "./${RUNTIME_DIR#/}" /out/; \
    fi

# =============================================================================
# runtime — small final image
# =============================================================================
FROM debian:bookworm-slim AS runtime

ARG PORT

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

# Run as a non-root user
RUN groupadd --system setback \
    && useradd --system --gid setback --no-create-home setback

WORKDIR /setback

# Binary at /setback/app, plus /setback/<RUNTIME_DIR> if one was given
COPY --from=builder --chown=setback:setback /out/ /setback/

USER setback

EXPOSE ${PORT}

ENTRYPOINT ["/setback/app"]