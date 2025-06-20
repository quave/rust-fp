FROM rust:1.75 as builder

WORKDIR /usr/src/frida
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libsqlite3-0 && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/frida/target/release/importer /usr/local/bin/importer
COPY --from=builder /usr/src/frida/target/release/processor /usr/local/bin/processor
COPY resources/init.sql /usr/local/bin/init.sql
COPY --from=builder /usr/src/frida/target/release/frida_ecom/config /usr/local/bin/frida_ecom/config

VOLUME /data

CMD ["importer"]