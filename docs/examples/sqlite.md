# SQLite example

## Prerequisites

`config/sqlite.toml` is the full SQLite demo. It uses `sqlite://crudo.db?mode=rwc`, `?` placeholders, `unixepoch()`, and setup triggers.

It requires `${WALLET_MNEMONIC}`, `${ALTCHA_SECRET}`, and `${ALTCHA_KEY_SECRET}`. The wallet passphrase is omitted and defaults to an empty string.

## Command

For a secret-free baseline, run the embedded starter:

```sh
crudo
```

## Explanation

The starter omits wallets. For full-demo use, inject a separately generated protected mnemonic and independent ALTCHA secrets through your runtime secret mechanism, then restrict CORS origins before exposure.
