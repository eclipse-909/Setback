# Start with the official Rust image to build your application
FROM rust:1.82 as builder

# Set the working directory to the root directory
WORKDIR /app

# Copy root files
COPY ./Cargo.docker.server.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock

# Copy resources workspace
COPY ./resources/src ./resources/src
COPY ./resources/Cargo.toml ./resources/Cargo.toml

# Copy server workspace
COPY ./server/src ./server/src
COPY ./server/Cargo.toml ./server/Cargo.toml

# Build the server in release mode
RUN cargo build -p server --release

# Use a minimal Debian image to run the compiled binary
FROM gcr.io/distroless/static-debian12

# Copy the built binary and .env file from the builder stage
COPY --from=builder /app/target/release/server /

# Expose the application port
EXPOSE 42069

# Start the application
CMD ["./server"]