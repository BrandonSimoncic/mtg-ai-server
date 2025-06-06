FROM rust:bookworm AS builder
WORKDIR /app
RUN apt-get update && apt install -y openssl
COPY . .
RUN cargo build --release --bin mtg_ai_server

FROM debian:bookworm-slim AS runtime
WORKDIR /app
RUN apt-get update && apt install -y openssl
COPY --from=builder /app/target/release/mtg_ai_server /usr/local/bin
ENTRYPOINT ["/usr/local/bin/mtg_ai_server"]
