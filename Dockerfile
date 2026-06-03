# syntax=docker/dockerfile:1.7
FROM rust:1.89-bookworm AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY xtask ./xtask

RUN --mount=type=cache,target=/usr/local/cargo/registry,id=cli-release-cargo-registry,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git/db,id=cli-release-cargo-git,sharing=locked \
    cargo build --locked --release -p cli-release-server --bin cli-release-server

FROM debian:bookworm-slim AS cli-release-server-runtime

ARG CLI_RELEASE_BUILD_COMMIT=unknown
ARG CLI_RELEASE_RUNTIME_UID=10001
ARG CLI_RELEASE_RUNTIME_GID=10001

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid "${CLI_RELEASE_RUNTIME_GID}" cli-release \
    && useradd \
        --uid "${CLI_RELEASE_RUNTIME_UID}" \
        --gid cli-release \
        --home-dir /data \
        --shell /usr/sbin/nologin \
        cli-release \
    && mkdir -p /data \
    && chown cli-release:cli-release /data

COPY --from=builder /app/target/release/cli-release-server /usr/local/bin/cli-release-server
RUN /usr/local/bin/cli-release-server --help >/dev/null

ENV CLI_RELEASE_BUILD_COMMIT=${CLI_RELEASE_BUILD_COMMIT}
WORKDIR /data
USER ${CLI_RELEASE_RUNTIME_UID}:${CLI_RELEASE_RUNTIME_GID}

EXPOSE 8080

ENTRYPOINT ["cli-release-server"]
