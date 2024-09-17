# Use the official Rust image as the build environment
FROM rust:1.70 as builder

# Set the working directory
WORKDIR /usr/src/pbft-blockchain

# Copy the Cargo.toml and Cargo.lock files
COPY Cargo.toml Cargo.lock ./

# Copy the source code
COPY . .

# Build the project in release mode
RUN cargo build --release

# Use a compatible Debian image with the required glibc version
FROM debian:bullseye-slim

# Install required libraries (if any)
RUN apt-get update && apt-get install -y \
    ca-certificates \
 && rm -rf /var/lib/apt/lists/*

# Copy the compiled binary from the builder stage
COPY --from=builder /usr/src/pbft-blockchain/target/release/pbft-blockchain /usr/local/bin/pbft-blockchain

# Set the working directory
WORKDIR /usr/local/bin

# Set the container entrypoint
ENTRYPOINT ["./pbft-blockchain"]
