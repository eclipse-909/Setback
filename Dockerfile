# syntax=docker/dockerfile:1

# This is for DEV and PROD
# Dockerfile for server and api
# Usage: See ./.github/scripts/cd-deploy.sh

# ARGs available to all stages
ARG PROFILE=dev
ARG PACKAGE
ARG PORT

# Stage 1: chef — base builder with cargo-chef installed
FROM rust:trixie AS chef
WORKDIR /build

COPY rust-toolchain.toml .
RUN rustup toolchain install

RUN curl -L --proto '=https' --tlsv1.2 \
	-sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
RUN cargo binstall --no-confirm cargo-chef

# Stage 2: planner — captures dependency graph into a recipe
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Stage 3: builder — cooks (pre-builds) dependencies, then builds the binary
FROM chef AS builder

ARG PROFILE
ARG PACKAGE

COPY --from=planner /build/recipe.json recipe.json

RUN cargo chef cook \
    $([ "$PROFILE" = "release" ] && echo "--release") \
    --recipe-path recipe.json

COPY . .
RUN cargo build \
    --profile "$PROFILE" \
    -p "$PACKAGE"

# Stage 4: runtime — distroless image
FROM gcr.io/distroless/cc-debian13:nonroot AS runtime

ARG PROFILE
ARG PACKAGE
ARG PORT

WORKDIR /setback

COPY --from=builder \
    /build/target/${PROFILE}/${PACKAGE} \
    /setback/service

USER setback
EXPOSE ${PORT}
ENTRYPOINT ["/setback/service"]