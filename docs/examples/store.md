# Store demo

`config/store.toml` is one universal digital-store bootstrap. It requires `DATABASE_URL`, `WALLET_MNEMONIC`, `ALTCHA_SECRET`, and `ALTCHA_KEY_SECRET`, sets `prefix = "v1"`, and selects its SQLite or PostgreSQL setup and SQL from `DATABASE_URL`.

## Run with SQLite

```sh
export DATABASE_URL='sqlite://crudo-store.db?mode=rwc'
read -rs 'WALLET_MNEMONIC?Wallet mnemonic: '; export WALLET_MNEMONIC
read -rs 'ALTCHA_SECRET?ALTCHA signing secret: '; export ALTCHA_SECRET
read -rs 'ALTCHA_KEY_SECRET?ALTCHA key-signing secret: '; export ALTCHA_KEY_SECRET
crudo --config config/store.toml
```

## Run with PostgreSQL

```sh
read -rs 'DATABASE_URL?PostgreSQL URL: '; export DATABASE_URL
read -rs 'WALLET_MNEMONIC?Wallet mnemonic: '; export WALLET_MNEMONIC
read -rs 'ALTCHA_SECRET?ALTCHA signing secret: '; export ALTCHA_SECRET
read -rs 'ALTCHA_KEY_SECRET?ALTCHA key-signing secret: '; export ALTCHA_KEY_SECRET
crudo --config config/store.toml
```

Open the [live store demo](https://demo-crudo.github.io/) and use `http://127.0.0.1:3000/v1` as its API URL. The configuration permits that hosted UI and local frontend development at `127.0.0.1:8000` or `localhost:8000`; custom deployments must set exact origins.

Startup creates the selected backend schema, then runs the common seed data in the same transaction. It idempotently adds four products and demo-only `admin` / `admin` when absent without overwriting edits. Registration and login require a fresh, one-time, IP-bound ALTCHA proof. Registration derives Base and Solana wallets.

::: warning Development only
Self-service top-ups create demo credit without payment-provider verification. Insufficient purchases return an x402 `402` requirement, but Crudo does not verify or settle payments. Change or remove the seeded administrator, do not expose top-ups, and use managed migrations and TLS across trust boundaries in production.
:::
