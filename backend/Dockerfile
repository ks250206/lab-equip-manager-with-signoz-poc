# syntax=docker/dockerfile:1
FROM rust:1.94.1-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
COPY src ./src
COPY migrations ./migrations
RUN cargo build --release --bins

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/equipment_reservation /usr/local/bin/equipment_reservation
COPY --from=builder /app/target/release/seed /usr/local/bin/seed
COPY migrations ./migrations
ENV BIND_ADDR=0.0.0.0:3000
EXPOSE 3000
CMD ["equipment_reservation"]
