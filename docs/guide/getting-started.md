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

Installed binaries do not need the source-tree `config/store.toml` bootstrap with `prefix = "v1"`. For production, select an explicit reviewed configuration, state its database URL and schema-management plan, and retain `127.0.0.1:3000` unless your network controls require another address. See the [store demo](/examples/store).

## Choose SQL placeholders

| Database | Placeholder style | Notes |
|---|---|---|
| SQLite and PostgreSQL | `$1`, `$2` | Universal bound SQL uses numbered placeholders for both engines; cast string path/query values where the selected SQL needs it. |

Crudo does not translate SQL. Use `{ sqlite = "...", postgres = "..." }` when backend syntax differs.

Next, learn the [core concepts](./core-concepts) and copy configuration into your project.
