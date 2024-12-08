# Stage 1: Dependency planning
FROM lukemathwalker/cargo-chef:latest-rust-1 AS planner
WORKDIR /app
# Copy workspace files first
COPY Cargo.toml Cargo.lock ./
COPY spectrum-offchain-ergo spectrum-offchain-ergo/
COPY src src/
RUN cargo chef prepare --recipe-path recipe.json

# Stage 2: Dependency caching and binary building
FROM lukemathwalker/cargo-chef:latest-rust-1 AS builder
WORKDIR /app
# Install necessary dependencies for dynamic linking
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    clang \
    libclang-dev \
    llvm-dev
# Copy the dependency plan from the planner stage
COPY --from=planner /app/recipe.json recipe.json
# Copy workspace files again for building
COPY Cargo.toml Cargo.lock ./
COPY spectrum-offchain-ergo spectrum-offchain-ergo/
# Build dependencies (cached layer)
RUN cargo chef cook --release --recipe-path recipe.json
# Copy source code and build dynamically linked binary
COPY . .
RUN cargo build --release --bin ergo-streaming

# Stage 3: Runtime environment
FROM debian:bookworm-slim AS runtime
WORKDIR /data

RUN mkdir -p /usr/conf 

# Install required runtime dependencies
RUN apt-get update && apt-get install -y \
    libssl3 \
    libstdc++6 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy only the binary
COPY --from=builder /app/target/release/ergo-streaming /bin/ergo-streaming

VOLUME /data
ENTRYPOINT ["/bin/ergo-streaming"]