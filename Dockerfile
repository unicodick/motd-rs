FROM rust:1.100-bookworm AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --locked

FROM debian:bookworm-slim
WORKDIR /app
COPY --from=build /src/target/release/motd-rs /usr/local/bin/motd-rs
ENTRYPOINT ["/usr/local/bin/motd-rs"]
