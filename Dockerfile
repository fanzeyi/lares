FROM rust:alpine as builder

ENV RUSTFLAGS="-C target-feature=-crt-static"

RUN apk update && apk add --no-cache openssl-dev musl-dev

WORKDIR /usr/src/binary
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && touch src/lib.rs
RUN cargo build --release
COPY . .
RUN touch src/lib.rs && cargo install --offline --path .

# ---------------------------------------------------------------------------- #

FROM alpine:latest

ENV LARES_HOST="0.0.0.0"
ENV LARES_PORT="4000"

RUN apk update && apk add --no-cache openssl ca-certificates libcurl libgcc
COPY --from=builder /usr/local/cargo/bin/lares /usr/local/bin/lares
COPY --from=builder /usr/src/binary/entrypoint.sh /usr/local/bin/entrypoint.sh

EXPOSE $LARES_PORT/tcp
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
CMD ["server"]
