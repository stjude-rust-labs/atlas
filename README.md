# atlas

## Prerequisites

  * [Rust](https://www.rust-lang.org/)
  * [PostgreSQL](https://www.postgresql.org/)
  * [sqlx-cli](https://github.com/launchbadge/sqlx/tree/main/sqlx-cli) (`cargo install sqlx-cli --no-default-features --features native-tls,postgres`)

## Development quickstart

atlas defines a development container manifest to quickly build an environment
for development and testing. This is compatible with, e.g., [GitHub Codespaces]
and [Visual Studio Code] with the [Dev Containers extension] installed.

After opening the project in the dev container,

```sh
cp .env.example .env
sqlx database setup --source atlas-server/migrations
cargo run -- server

# open <local-address>/docs
```

[GitHub Codespaces]: https://github.com/features/codespaces
[Visual Studio Code]: https://code.visualstudio.com/
[Dev Containers extension]: https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers
