FROM rust:latest as builder
WORKDIR /usr/src/binary
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && touch src/lib.rs
RUN cargo build --release
COPY . .
RUN touch src/lib.rs && cargo install --offline --path .

FROM debian:buster-slim
RUN apt-get update && apt-get install -y openssl ca-certificates libcurl4 && update-ca-certificates
COPY --from=builder /usr/local/cargo/bin/lares /usr/local/bin/lares
COPY --from=builder /usr/src/binary/entrypoint.sh /usr/local/bin/entrypoint.sh
ENTRYPOINT ["entrypoint.sh"]
CMD ["server"]