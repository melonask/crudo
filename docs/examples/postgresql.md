# PostgreSQL example

`config/postgres.toml` reads `DATABASE_URL`, uses `$1`-style placeholders, casts externally supplied numeric strings (for example `$1::BIGINT`), and uses `EXTRACT(EPOCH FROM now())`. It also maintains `wallet_counters` to serialize per-user/profile address allocation.

```sh
DATABASE_URL='postgres://user:password@localhost:5432/crudo' \
WALLET_MNEMONIC='stored-by-your-secret-manager' \
ALTCHA_SECRET='stored-by-your-secret-manager' \
ALTCHA_KEY_SECRET='stored-by-your-secret-manager' \
  crudo --config config/postgres.toml
```

The literal mnemonic value above is a label, not a seed phrase. Use TLS for database transport when it crosses a trust boundary.
