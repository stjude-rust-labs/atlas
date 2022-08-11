# altas

## Prerequisites

  * [Rust](https://www.rust-lang.org/)
  * [PostgreSQL](https://www.postgresql.org/)
  * [sqlx-cli](https://github.com/launchbadge/sqlx/tree/main/sqlx-cli) (`cargo install sqlx-cli --no-default-features --features native-tls,postgres`)

## Development quickstart

```sh
git clone https://github.com/stjude-rust-labs/atlas.git
cd atlas

cp .env.example .env

docker compose up --detach
sqlx database setup
docker container exec --interactive atlas-db-1 psql --username postgres atlas < tests/sql/seeds.sql

cargo run -- run

curl http://localhost:3000/samples
```
