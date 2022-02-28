# altas

## Prerequisites

  * [Rust](https://www.rust-lang.org/)
  * [PostgreSQL](https://www.postgresql.org/)
  * [sqlx-cli](https://github.com/launchbadge/sqlx/tree/master/sqlx-cli) (`cargo install sqlx-cli --no-default-features --features native-tls,postgres`)

## Development quickstart

```sh
git clone https://github.com/stjude-rust-labs/atlas.git
cd atlas

cp .env.example .env

docker container run --detach --rm --name atlas_postgres --publish 5432:5432 --env POSTGRES_PASSWORD=dev postgres:14.2
sqlx database setup
docker container exec --interactive atlas_postgres psql --username postgres atlas < tests/sql/seeds.sql

cargo run -- run

curl http://localhost:3000/samples
```
