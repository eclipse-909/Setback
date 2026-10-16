# syntax=docker/dockerfile:1

# This is for DEV and PROD
# Dockerfile for server and api
# Usage: See ./.github/scripts/cd-deploy.sh

# =============================================================================
# ARGs available to all stages
# =============================================================================
ARG PROFILE=debug
ARG PACKAGE
ARG PORT

# =============================================================================
# Stage 1: chef — base builder with cargo-chef installed
# =============================================================================
FROM rust:bookworm AS chef
WORKDIR /build

# Install the toolchain from rust-toolchain.toml in its own cached layer.
# This layer only rebuilds when that file changes.
COPY rust-toolchain.toml .
RUN rustup toolchain install

RUN curl -L --proto '=https' --tlsv1.2 \
	-sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
RUN cargo binstall --no-confirm cargo-chef

# =============================================================================
# Stage 2: planner — captures dependency graph into a recipe
# =============================================================================
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# =============================================================================
# Stage 3: builder — cooks (pre-builds) dependencies, then builds the binary
#
# Dependencies are built in their own layer so that they are cached by Docker
# as long as Cargo.toml / Cargo.lock don't change. Only your source code
# changes will trigger a re-compile of your own crates.
# =============================================================================
FROM chef AS builder

ARG PROFILE
ARG PACKAGE

COPY --from=planner /build/recipe.json recipe.json

# Cook dependencies only — this layer is cached until Cargo.{toml,lock} change
RUN cargo chef cook \
    $([ "$PROFILE" = "release" ] && echo "--release") \
    --recipe-path recipe.json

# Now bring in the full source and build the selected package
COPY . .
RUN cargo build \
    --profile "$PROFILE" \
    -p "$PACKAGE"

# =============================================================================
# Stage 4: runtime — small final image
# =============================================================================
FROM debian:bookworm-slim AS runtime

ARG PROFILE
ARG PACKAGE
ARG PORT

# Install only the minimal runtime libraries needed by a typical Rust binary.
# Remove the apt cache afterwards to keep the layer small.
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

# Run as a non-root user
RUN groupadd --system setback \
    && useradd --system --gid setback --no-create-home setback

WORKDIR /setback

# Copy the compiled binary from the builder stage.
# The output directory is target/release for --profile release,
# and target/debug for --profile debug (the default).
COPY --from=builder --chown=setback:setback \
    /build/target/${PROFILE}/${PACKAGE} \
    /setback/app

USER setback

EXPOSE ${PORT}

ENTRYPOINT ["/setback/app"]