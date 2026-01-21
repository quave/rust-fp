ARG PROFILE=release
ARG MODULE=ecom-f2
ARG FRIDA_ENV=production
ARG YQ_PLATFORM=linux_arm64

# # build the web app
FROM node:alpine AS web-builder

WORKDIR /app
COPY web/package*.json ./
RUN npm ci

COPY web/ ./
RUN npm run build

# build the rust binaries
FROM rust:slim-bookworm AS rust-builder

ARG PROFILE
ARG MODULE
ARG FRIDA_ENV
ARG YQ_PLATFORM

RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev wget &&\
    rm -rf /var/lib/apt/lists/* && \
    wget https://github.com/mikefarah/yq/releases/download/v4.2.0/yq_${YQ_PLATFORM} -O /usr/local/bin/yq && \
    chmod +x /usr/local/bin/yq

WORKDIR /app

COPY . .
RUN yq eval-all 'select(fileIndex==0) * select(fileIndex==1) * select(fileIndex==2)' \
    /app/config/base.yaml /app/config/${FRIDA_ENV}.yaml /app/${MODULE}/config/${FRIDA_ENV}.yaml \
    > /app/config/total_config.yaml && \
    cargo build --${PROFILE}

# # build the final image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates libssl3 &&\
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=rust-builder /app/config/total_config.yaml /app/config/total_config.yaml
COPY --from=rust-builder /app/target/release/importer /app/
COPY --from=rust-builder /app/target/release/processor /app/
COPY --from=rust-builder /app/target/release/backend /app/
COPY --from=web-builder /app/dist /app/web/dist

CMD ["./importer --config /app/config/total_config.yaml"]