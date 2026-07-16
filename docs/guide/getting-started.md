# Getting started

## Start without secrets

Install and run the built-in starter first.

```sh
cargo install crudo
crudo
```

In another terminal, verify the API and create an item:

```sh
curl http://127.0.0.1:3000/v1/health
curl -X POST http://127.0.0.1:3000/v1/items \
  -H 'content-type: application/json' \
  -d '{"name":"first item"}'
```

Bare `crudo` uses the embedded `config/minimal.toml` starter. It writes `crudo.db` in the current writable directory, listens on loopback for the native CLI, and exposes `/v1/health` plus item list/create/get/update/delete routes.

Setup statements run in one transaction before listening. A setup failure rolls them all back.

## Select a configuration

`--config` accepts a local path or HTTPS URL. `--address` overrides `[server].address`.

```sh
# Select a custom local or HTTPS configuration.
crudo --config path/to/config.toml
crudo --config https://config.example.com/crudo.toml
```

## Choose SQL placeholders

| Database | Placeholder style | Notes |
|---|---|---|
| SQLite | `?` | Use SQLite SQL syntax. |
| PostgreSQL | `$1`, `$2` | Cast string path/query values before comparing them to numeric columns. |

Next, learn the [core concepts](./core-concepts) and copy configuration into your project.
