# Production image for a Doido app. Build the release binary, then run it behind
# Thruster (an HTTP/2 proxy) for TLS termination and X-Sendfile acceleration.
FROM rust:1-slim AS build
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:stable-slim
WORKDIR /app
COPY --from=build /app/target/release/{doido_name} /usr/local/bin/app
COPY --from=build /app/config ./config
COPY --from=build /app/app ./app
EXPOSE 3000
# Wrap with `thrust` when present for HTTP/2 + asset acceleration.
CMD ["app", "server"]
