FROM rust:1.85-slim AS builder
WORKDIR /build
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY src/ src/
COPY benches/ benches/
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /build/target/release/forjar /usr/local/bin/forjar
ENTRYPOINT ["forjar"]
