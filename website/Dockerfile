# Start with the official Rust image to build your application
FROM rust:1.82 as builder

# Set the working directory to the root directory
WORKDIR /app

# Copy root files
COPY ./Cargo.docker.website.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock

# Copy resources workspace
COPY ./resources/src ./resources/src
COPY ./resources/Cargo.toml ./resources/Cargo.toml

# Copy website workspace
COPY ./website/client ./website/client
COPY ./website/src ./website/src
COPY ./website/Cargo.toml ./website/Cargo.toml
COPY ./website/.env.release ./website/.env.release
COPY ./website/schema.sql ./website/schema.sql

# Build the website in release mode
RUN cargo build -p website --release

# Remove src, Cargo.toml, and schema.sql from ./website/ for easy copying to runtime image
RUN rm -rf ./website/src ./website/Cargo.toml ./website/schema.sql

# Use a distroless image to run the compiled binary
FROM gcr.io/distroless/cc-debian12

# Copy the built binary and website directory from the builder stage
COPY --from=builder /app/target/release/website /setback-website
COPY --from=builder /app/website /website

# Expose the application port
EXPOSE 8080

# Start the application
ENTRYPOINT ["./setback-website"]