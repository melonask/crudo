# PostgreSQL example

## Prerequisites

`config/postgres.toml` reads `DATABASE_URL`, uses `$1`-style placeholders, and casts externally supplied numeric strings such as `$1::BIGINT`. It uses `EXTRACT(EPOCH FROM now())` and maintains `wallet_counters` to serialize per-user/profile address allocation.

## Command

Provide the required environment values and run the configuration:

```sh
DATABASE_URL='postgres://user:password@localhost:5432/crudo' \
WALLET_MNEMONIC='stored-by-your-secret-manager' \
ALTCHA_SECRET='stored-by-your-secret-manager' \
ALTCHA_KEY_SECRET='stored-by-your-secret-manager' \
  crudo --config config/postgres.toml
```

## Explanation

The literal mnemonic value is a label, not a seed phrase. Use TLS for database transport when it crosses a trust boundary.
