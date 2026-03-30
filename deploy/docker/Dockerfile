FROM rust:latest AS builder

WORKDIR /app

# Cache dependencies — only re-download/compile when Cargo.toml or Cargo.lock change
COPY Cargo.toml Cargo.lock ./
COPY build.rs ./
COPY rust-toolchain.toml ./

# Create stub src/lib.rs and bins so cargo can resolve the dependency tree
RUN mkdir -p src/bin && \
    echo "fn main() {}" > src/bin/controller.rs && \
    echo "fn main() {}" > src/bin/agent.rs && \
    echo "" > src/lib.rs && \
    mkdir -p web/js web/css web/partials/views migrations && \
    touch web/index.html web/css/style.css && \
    echo "-- version: 1\n-- description: stub" > migrations/0001_init.sql

# Build dependencies only (cached layer)
RUN cargo build --release 2>/dev/null || true

# Now copy the real source and rebuild (only our code recompiles)
COPY src/ src/
COPY web/ web/
COPY migrations/ migrations/
COPY examples/ examples/
RUN mkdir -p scripts

# Touch files to invalidate the build cache for our crates only
RUN touch src/lib.rs src/bin/controller.rs src/bin/agent.rs
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
