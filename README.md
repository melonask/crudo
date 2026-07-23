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

The source tree includes one universal digital-store configuration: `config/store.toml`. Set `DATABASE_URL` to either a SQLite or PostgreSQL URL, plus `WALLET_MNEMONIC`, `ALTCHA_SECRET`, and `ALTCHA_KEY_SECRET`; then run `crudo --config config/store.toml`. See the [store guide](https://melonask.github.io/crudo/examples/store). It is a development bootstrap, not payment processing or a deployment template.

## License

[MIT](LICENSE)
