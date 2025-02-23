FROM rust:latest as builder

WORKDIR /usr/src/hft_engine

# Copy the entire project
COPY . .

# Build the main application
RUN cargo build --release

FROM ubuntu:latest

# Install necessary runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    openssl \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary
COPY --from=builder /usr/src/hft_engine/target/release/hft_engine /usr/local/bin/hft_engine

# Set working directory
WORKDIR /usr/local/bin

# Run the main application
CMD ["hft_engine"]