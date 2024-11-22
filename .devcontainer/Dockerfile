# syntax=docker/dockerfile:1

FROM mcr.microsoft.com/devcontainers/base:debian-12

ENV PATH=/home/vscode/.cargo/bin:${PATH}

RUN <<EOS
apt-get update
apt-get install --yes ca-certificates libssl-dev pkg-config postgresql-client
rm -rf /var/lib/apt/lists/*
EOS

USER vscode

RUN curl https://sh.rustup.rs -sSf | sh -s -- \
    -y \
    --profile minimal \
    --default-toolchain stable \
    --component clippy,rustfmt

RUN cargo install \
    --no-default-features \
    --features native-tls,postgres \
    --version 0.8.2 \
    sqlx-cli
