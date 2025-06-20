FROM rust:1.75 as builder

WORKDIR /usr/src/frida

# Create a dummy project to cache dependencies
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src frida_core/src frida_ecom/src/bin
COPY frida_core/Cargo.toml frida_core/
COPY frida_ecom/Cargo.toml frida_ecom/
RUN echo "fn main() {}" > src/main.rs && \
  echo "" > frida_core/src/lib.rs && \
  echo "fn main() {}" > frida_ecom/src/bin/importer.rs && \
  echo "fn main() {}" > frida_ecom/src/bin/processor.rs && \
  cargo build --release
RUN rm -rf src && rm -rf frida_core && rm -rf frida_ecom

# Now copy the real source and build
COPY . .
# Add debug output
RUN rm -rf target/release/deps/frida_* && \
  rm -rf target/release/build/frida_* && \
  rm -rf target/release/deps/libfrida_* && \
  rm -rf target/release/.fingerprint/frida_* && \
  RUST_BACKTRACE=1 cargo build --release -v

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libsqlite3-0 sqlite3 && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /usr/src/frida/target/release/importer /usr/local/bin/importer
COPY --from=builder /usr/src/frida/target/release/processor /usr/local/bin/processor
COPY --from=builder /usr/src/frida/target/release/frida_ecom/config/ /usr/local/bin/frida_ecom/config
COPY resources/init.sql /app/init.sql

RUN sqlite3 /app/stage.sqlite3 < /app/init.sql

CMD ["importer"]