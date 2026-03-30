# syntax=docker/dockerfile:1

# NOTICE!
# See ./prod/setback_server/Dockerfile for the production Dockerfile

# ── Build stage ───────────────────────────────────────────────────────────────
FROM rust:latest AS builder

WORKDIR /app

# Copy workspace manifests first for better layer caching
COPY Cargo.toml Cargo.lock ./
COPY libsetback/Cargo.toml ./libsetback/
COPY setback/Cargo.toml ./setback/
COPY setback_server/Cargo.toml ./setback_server/
COPY setback_website/Cargo.toml ./setback_website/
COPY setbackctl/Cargo.toml ./setbackctl/

# Create dummy source files so cargo can cache dependencies
RUN mkdir -p setback_server/src setback_website/src \
    && touch libsetback/src/lib.rs \
    && echo "fn main() {}" > setback/src/main.rs \
    && echo "fn main() {}" > setback_server/src/main.rs \
    && echo "fn main() {}" > setback_website/src/main.rs \
    && echo "fn main() {}" > setbackctl/src/main.rs

RUN cargo +nightly build -p setback_website
RUN rm -rf libsetback/src setback/src setback_server/src setback_website/src setbackctl/src

# Copy real source and build for real
COPY libsetback/src    ./libsetback/src
COPY setback/src       ./setback/src
COPY setback_server/src ./setback_server/src
COPY setback_website/src ./setback_website/src
COPY setbackctl/src ./setbackctl/src

# Touch main.rs so cargo knows to recompile
RUN touch setback_website/src/main.rs
RUN cargo +nightly build -p setback_website

# ── Runtime stage ─────────────────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/debug/setback_website ./setback_website
COPY setback_website/frontend ./frontend

EXPOSE 8080

CMD ["./setback_website"]
