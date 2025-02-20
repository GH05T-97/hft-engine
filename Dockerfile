FROM rust:latest as builder

WORKDIR /usr/src/hft_engine

# Copy the entire project
COPY . .

# Build the example explicitly
RUN cargo build --release --example futures_connect_test

FROM debian:bullseye-slim

# Install necessary runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy the example binary (note the path change)
COPY --from=builder /usr/src/hft_engine/target/release/examples/futures_connect_test /usr/local/bin/futures_connect_test

# Set working directory
WORKDIR /usr/local/bin

# Set the example as the entrypoint
CMD ["futures_connect_test"]