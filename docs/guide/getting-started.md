# Getting started

Clone the repository, then run from its root. The CLI defaults to `config/minimal.toml`; `--config` accepts a local path or HTTPS URL and `--address` overrides `[server].address`. Install the binary with `cargo install crudo` (or `brew install melonask/crudo/crudo`), but those installation methods do not place the repository configuration in your current directory.

## Start without secrets

The checked-in full demo uses wallets and secrets. The repository's `config/minimal.toml` is a no-environment local API:

```sh
crudo
curl http://127.0.0.1:3000/v1/health
curl -X POST http://127.0.0.1:3000/v1/items -H 'content-type: application/json' -d '{"name":"first item"}'
```

It creates SQLite data, exposes `/v1/health`, and provides item list/create/get/update/delete routes. Setup statements run in one transaction before listening; a failure rolls them all back.

Next: learn the [core concepts](./core-concepts) and copy the configuration into your project. SQLite uses `?` parameters; PostgreSQL uses `$1`, `$2`, and explicit casts for string path/query values compared to numeric columns.
