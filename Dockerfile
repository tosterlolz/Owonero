# Multi-stage build for Owonero
# Builder stage: compile the Rust project
FROM rust:1.70-slim as builder

# Install system deps needed for openssl and building
RUN apt-get update && \
    apt-get install -y --no-install-recommends pkg-config libssl-dev build-essential ca-certificates git && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/owonero

# Cache dependencies by copying Cargo files first
COPY Cargo.toml Cargo.lock ./
# If workspace, adjust as needed
RUN mkdir -p src && echo "fn main() {}" > src/main.rs && cargo fetch

# Copy source and build
COPY . .

RUN cargo build --release

# Runtime image
FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/owonero/target/release/owonero-rs /usr/local/bin/owonero

EXPOSE 6969 6767
ENTRYPOINT ["/usr/local/bin/owonero"]
