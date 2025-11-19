FROM node:alpine as web-builder

WORKDIR /app
COPY web/package*.json ./
RUN npm ci

COPY web/ ./
RUN npm run build

FROM rust:slim-bookworm as rust-builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# COPY Cargo.toml Cargo.lock ./
# COPY common/Cargo.toml common/
# COPY processing/Cargo.toml processing/
# COPY ecom/Cargo.toml ecom/
# COPY ecom/build.rs ecom/

# RUN mkdir -p \
#     common/src \
#     processing/src \
#     ecom/src \
#     && touch \
#     common/src/lib.rs \
#     processing/src/lib.rs \
#     ecom/src/lib.rs

# RUN cargo build --release

COPY . .
COPY --from=web-builder /app/dist web/dist

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=rust-builder /app/target/release/config/total_config.yaml /app/config/total_config.yaml
COPY --from=rust-builder /app/target/release/importer /app/
COPY --from=rust-builder /app/target/release/processor /app/
COPY --from=rust-builder /app/target/release/backend /app/
COPY --from=web-builder /app/dist /app/web/dist

ENV RUST_LOG=info

CMD ["./importer --config /app/config/total_config.yaml"]