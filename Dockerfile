FROM rust:1.75 as builder

WORKDIR /usr/src/hft_engine
COPY . .
RUN cargo build --release

FROM debian:bullseye-slim
COPY --from=builder /usr/src/hft_engine/target/release/hft_engine /usr/local/bin/hft_engine
COPY --from=builder /usr/src/hft_engine/target/release/examples/* /usr/local/bin/
CMD ["futures_connect_test"]