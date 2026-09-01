# Production image for a Doido app. Multi-stage build: compile the release
# binary, then copy it into a minimal distroless runtime (glibc for dynamic Rust
# binaries). Wrap with Thruster at deploy time for HTTP/2 + asset acceleration.
FROM rust:1-bookworm AS build
WORKDIR /app

# Cache dependency compilation when Cargo.toml / Cargo.lock are unchanged.
COPY Cargo.toml Cargo.lock* ./
RUN mkdir -p src \
    && echo "fn main() {}" > src/main.rs \
    && cargo build --release \
    && rm -rf src

COPY . .
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12:nonroot
WORKDIR /app
COPY --from=build /app/target/release/{doido_name} /app/app
COPY --from=build /app/config ./config
COPY --from=build /app/app ./app
USER nonroot
EXPOSE 8080
CMD ["/app/app", "server"]
