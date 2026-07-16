<p align="center"><img src="logo.svg" width="96" alt="crudo logo"></p>

# crudo

[![CI](https://github.com/melonask/crudo/actions/workflows/ci.yml/badge.svg)](https://github.com/melonask/crudo/actions/workflows/ci.yml)

Configuration-driven JSON APIs backed by SQL: declare routes, authentication, protections, schema setup, and response shapes in TOML.

[Documentation](https://melonask.github.io/crudo/) · [Repository](https://github.com/melonask/crudo)

## Install

```sh
cargo install crudo
# or: brew install melonask/crudo/crudo
```

## Zero-environment quick start (from a clone)

Clone this repository and run these commands from its root. The CLI defaults to the checked-in `config/minimal.toml`; a package installation does not copy that configuration into an arbitrary current directory. The minimal configuration has no wallet or secret environment requirements:

```sh
crudo
curl http://127.0.0.1:3000/v1/health
curl -X POST http://127.0.0.1:3000/v1/items -H 'content-type: application/json' -d '{"name":"first item"}'
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

## Included protections

Parameter binding, Argon2 password hashing, Basic/Bearer authentication, ALTCHA, CORS, request body/time/concurrency limits, per-IP rate limits, and transactional schema/wallet stages are available by configuration.

The full SQLite/PostgreSQL demos include wallet stages. `WALLET_MNEMONIC` is **not** globally required: `[wallets]` is optional, but `${WALLET_MNEMONIC}` makes it required for those full-demo files. Omit `[wallets]` and wallet stages to run without it. See the [full documentation](https://melonask.github.io/crudo/) for configuration, Docker, operations, wallet profiles, and API examples.

## License

[MIT](LICENSE)
