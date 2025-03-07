# atlas

**atlas** provides tools for gene expression analyses.

It is split into a command-line application and server.

## CLI

The CLI includes commands to perform gene expression quantification
(`quantify`), normalize counts (`normalize`), and cluster raw counts using
t-SNE (`transform`).

### Prerequisites

* [Rust](https://www.rust-lang.org/)

### Build

```console
$ cargo build --release --package atlas-cli
```

By default, the binary is written to `target/release/atlas`.

### Run

See `atlas --help` for more information.

## Server

The server includes a REST API over HTTP. It is an optional component that can
be used to store quantifications in an associated database.

### Prerequisites

* [Rust](https://www.rust-lang.org/)
* [PostgreSQL](https://www.postgresql.org/)
* [sqlx-cli](https://github.com/launchbadge/sqlx/tree/main/sqlx-cli) (`cargo install sqlx-cli --no-default-features --features native-tls,postgres`)

### Build

```console
$ cargo build --release --package atlas-server
```

By default, the binary is written to `target/release/atlas-server`.

### Run

```console
$ sqlx database setup --source atlas-server/migrations
$ atlas-server
```

OpenAPI documentation can then be viewed at `http://<local-address>/docs`.

## Development quickstart

atlas defines a development container manifest to quickly build an environment
for development and testing. This is compatible with, e.g., [GitHub Codespaces]
and [Visual Studio Code] with the [Dev Containers extension] installed.

For server development,

```sh
cp .env.example .env
sqlx database setup --source atlas-server/migrations
cargo run -- server

# open <local-address>/docs
```

## Legal

This project is licensed as either [Apache 2.0][license-apache] or [MIT][license-mit] at
your discretion. Additionally, please see
[the disclaimer](https://github.com/stjude-rust-labs#disclaimer) that applies to all
crates and command line tools made available by St. Jude Rust Labs.

[GitHub Codespaces]: https://github.com/features/codespaces
[Visual Studio Code]: https://code.visualstudio.com/
[Dev Containers extension]: https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers
[license-apache]: ./LICENSE-APACHE
[license-mit]: ./LICENSE-MIT
