FROM rust:alpine3.20 as build
WORKDIR /build
COPY ./* .
RUN cargo test
RUN cargo build --release -o app

FROM alpine:3.20 as run
WORKDIR /app
COPY --from=build /build/cfg/application.yml /app/cfg.yml
COPY --from=build /build/target/release/app /app/service
ENV CONFIG=/app/cfg.yml
CMD ["service"]
