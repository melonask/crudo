<p align="center"><img src="logo.svg" width="96" alt="crudo logo"></p>

# crudo

[![CI](https://github.com/melonask/crudo/actions/workflows/ci.yml/badge.svg)](https://github.com/melonask/crudo/actions/workflows/ci.yml)

Configuration-driven JSON APIs backed by SQL: declare routes, authentication, protections, schema setup, and response shapes in TOML.

[Documentation](https://melonask.github.io/crudo/) · [Repository](https://github.com/melonask/crudo)

## Quick start

```sh
cargo install crudo
crudo
# Optional: in another terminal
curl http://127.0.0.1:3000/v1/health
```

The starter:

- Writes `crudo.db` in the current writable directory.
- Listens on loopback for the native CLI.

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

Configure these protections as needed:

- Parameter binding and Argon2 password hashing
- Basic/Bearer authentication and ALTCHA
- CORS, body, timeout, concurrency, and per-IP limits
- Transactional schema and wallet stages

The full SQLite/PostgreSQL demos include wallet stages. `WALLET_MNEMONIC` is **not** globally required: `[wallets]` is optional, but `${WALLET_MNEMONIC}` makes it required for those full-demo files. Omit `[wallets]` and wallet stages to run without it.

See the [full documentation](https://melonask.github.io/crudo/) for configuration, Docker, operations, wallet profiles, and API examples.

## License

[MIT](LICENSE)
