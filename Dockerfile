FROM rust:latest as builder

WORKDIR /usr/src/hft_engine

# Copy the entire project
COPY . .

# Build the main binary instead of the example
RUN cargo build --release

FROM debian:bullseye-slim

# Install necessary runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy the main binary (note the path change)
COPY --from=builder /usr/src/hft_engine/target/release/hft_engine /usr/local/bin/hft_engine

# Set working directory
WORKDIR /usr/local/bin

# Set the main binary as the entrypoint
CMD ["hft_engine"]