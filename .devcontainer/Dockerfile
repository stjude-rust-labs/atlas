# syntax=docker/dockerfile:1.4

FROM mcr.microsoft.com/devcontainers/base:debian-12

ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse \
    PATH=/home/vscode/.cargo/bin:${PATH}

RUN <<EOS
apt-get update
apt-get install --yes libssl-dev pkg-config postgresql-client
rm -rf /var/lib/apt/lists/*
EOS

USER vscode

RUN <<EOS
curl https://sh.rustup.rs -sSf | sh -s -- -y --profile minimal
rustup toolchain install stable
rustup default stable
rustup component add clippy rustfmt
EOS

RUN cargo install \
    --no-default-features \
    --features native-tls,postgres \
    --version 0.6.3 \
    sqlx-cli
