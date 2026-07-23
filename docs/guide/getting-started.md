# Getting started

## Smallest API

Install crudo and create `./Crudo.toml`:

```sh
cargo install crudo
```

```toml
[[endpoints]]
method = "GET"
path = "/health"
action = "health"

[actions.health]
sql = "SELECT 'ok' AS status"
result = "one"
```

```sh
crudo
curl http://127.0.0.1:3000/health
```

The omitted `[database]` uses `sqlite://crudo.db?mode=rwc`; omitted setup executes no statements. The omitted `[server]` uses `127.0.0.1:3000` and no prefix, so the route is exactly `/health`.

## Configuration selection

`--config` selects a local path or HTTPS URL. Otherwise, crudo reads `./Crudo.toml`. If neither is available, startup fails with guidance; malformed or unreadable selected configuration also fails startup. `--address` overrides `[server].address`.

```sh
# Select a custom local or HTTPS configuration explicitly.
crudo --config path/to/config.toml
crudo --config https://config.example.com/crudo.toml
```

Installed binaries do not need a repository-relative `config/sqlite.toml`; it is a source-tree store bootstrap with `prefix = "v1"`. For production, select an explicit reviewed configuration, state its database URL and schema-management plan, and retain `127.0.0.1:3000` unless your network controls require another address. See the [SQLite store demo](/examples/sqlite) or [PostgreSQL store demo](/examples/postgresql).

## Choose SQL placeholders

| Database | Placeholder style | Notes |
|---|---|---|
| SQLite | `?` | Use SQLite SQL syntax. |
| PostgreSQL | `$1`, `$2` | Cast string path/query values before comparing them to numeric columns. |

Next, learn the [core concepts](./core-concepts) and copy configuration into your project.
