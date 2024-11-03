# Chef planner stage
FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

RUN apt-get update && apt-get -y upgrade && apt-get install -y libclang-dev pkg-config

# Recipe creation
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Builder stage with dependencies cached
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json

# Build dependencies - this layer is cached as long as dependencies don't change
RUN cargo chef cook --release --recipe-path recipe.json

# # Build application
COPY . .
RUN cargo build --release

# Runtime stage
FROM ubuntu as runtime
WORKDIR /app

COPY --from=builder /app/target/release/reth-db-reader /app/reth-db-reader
ENV RETH_DB_PATH=/root/.local/share/reth/mainnet

CMD ["/app/reth-db-reader"]