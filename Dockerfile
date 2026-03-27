FROM rust:latest AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src/ src/
COPY examples/ examples/
COPY scripts/ scripts/
RUN mkdir -p scripts

RUN cargo build --release

FROM debian:trixie-slim

RUN apt-get update && apt-get install -y ca-certificates libssl3 curl \
    mosquitto-clients amqp-tools redis-tools \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/kronforce /usr/local/bin/kronforce
COPY --from=builder /app/target/release/kronforce-agent /usr/local/bin/kronforce-agent
COPY --from=builder /app/examples/ /opt/kronforce/examples/

RUN mkdir -p /data /scripts

ENV KRONFORCE_DB=/data/kronforce.db
ENV KRONFORCE_BIND=0.0.0.0:8080
ENV KRONFORCE_SCRIPTS_DIR=/scripts
ENV RUST_LOG=kronforce=info

EXPOSE 8080

CMD ["kronforce"]
