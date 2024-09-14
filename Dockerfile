FROM rust:slim-bullseye as build
WORKDIR /build

COPY ./cfg ./cfg
COPY ./migrations ./migrations
COPY ./src ./src
COPY ./build.rs .
COPY Cargo.toml .
COPY Cargo.lock .

ARG PG_HOST
ARG PG_PORT
ARG PG_DB
ENV PG_HOST $PG_HOST
ENV PG_PORT $PG_PORT
ENV PG_DB $PG_DB

RUN apt-get update -y
RUN apt-get install -y libssl-dev pkg-config
RUN cargo install sqlx-cli refinery_cli
RUN --mount=type=secret,required=true,id=pg_user,target=/run/secrets/pg_user \
    --mount=type=secret,required=true,id=pg_pass,target=/run/secrets/pg_pass \
    PG_USER=$(cat /run/secrets/pg_user) \
    PG_PASS=$(cat /run/secrets/pg_pass) \
    DATABASE_URL=postgres://$PG_USER:$PG_PASS@$PG_HOST:$PG_PORT/$PG_DB && \
    export DATABASE_URL && \
    refinery migrate -e DATABASE_URL -p ./migrations && \
    cargo sqlx prepare
ENV SQLX_OFFLINE=true
RUN cargo test
RUN cargo build --release

FROM debian:bullseye-slim as run
WORKDIR /app
COPY --from=build /build/cfg/compose.yml /app/cfg.yml
COPY --from=build /build/target/release/otus_highload_social_network_1 /app/service
ENV CONFIG=/app/cfg.yml
COPY with_file_env_entrypoint.sh with_file_env.sh
EXPOSE 8080
ENTRYPOINT ["/app/with_file_env.sh", "/app/service"]