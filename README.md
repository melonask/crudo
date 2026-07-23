<p align="center"><img src="logo.svg" width="96" alt="crudo logo"></p>

# crudo

[![CI](https://github.com/melonask/crudo/actions/workflows/ci.yml/badge.svg)](https://github.com/melonask/crudo/actions/workflows/ci.yml)

Configuration-driven JSON APIs backed by SQL: declare routes, authentication, protections, schema setup, and response shapes in TOML.

[Documentation](https://melonask.github.io/crudo/) · [Repository](https://github.com/melonask/crudo)

## Quick start

```sh
cargo install crudo
```

Create `./Crudo.toml`:

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

This configuration uses local `sqlite://crudo.db?mode=rwc`, `127.0.0.1:3000`, and no prefix, so its endpoint is exactly `/health`. See the [full documentation](https://melonask.github.io/crudo/) for configuration and operations.

## Optional store demo

The source tree includes SQLite and PostgreSQL digital-store configurations. Use the [live store demo](https://demo-crudo.github.io/) with its visible API URL field (default: `http://127.0.0.1:3000/v1`), or start with the [SQLite](https://melonask.github.io/crudo/examples/sqlite) or [PostgreSQL](https://melonask.github.io/crudo/examples/postgresql) guides. They are development bootstraps, not payment processing or deployment templates.

## License

[MIT](LICENSE)
