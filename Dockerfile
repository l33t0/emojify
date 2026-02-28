FROM rust:slim AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
RUN mkdir src && \
    echo '//! stub' > src/lib.rs && \
    echo 'fn main() {}' > src/main.rs && \
    cargo build --release && \
    rm -rf src

COPY src/ src/
COPY assets/ assets/

RUN touch src/main.rs src/lib.rs && \
    cargo build --release && \
    strip /app/target/release/emojify

FROM debian:bookworm-slim

COPY --from=builder /app/target/release/emojify /usr/local/bin/emojify

ENTRYPOINT ["/usr/local/bin/emojify"]
