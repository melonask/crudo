# PostgreSQL example

## Prerequisites

`config/postgres.toml` reads `DATABASE_URL`, uses `$1`-style placeholders, and casts externally supplied numeric strings such as `$1::BIGINT`. It uses `EXTRACT(EPOCH FROM now())` and maintains `wallet_counters` to serialize per-user/profile address allocation.

## Command

Export `DATABASE_URL` and load the three secrets through your deployment's secret provider:

```sh
export DATABASE_URL='postgres://user:password@localhost:5432/crudo'
# Required secret-provider exports:
# WALLET_MNEMONIC, ALTCHA_SECRET, ALTCHA_KEY_SECRET
crudo --config config/postgres.toml
```

## Explanation

Use TLS for database transport when it crosses a trust boundary.
