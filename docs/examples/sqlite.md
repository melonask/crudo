# SQLite example

`config/sqlite.toml` is the full SQLite demo. It uses `sqlite://crudo.db?mode=rwc`, `?` placeholders, `unixepoch()`, and setup triggers. It requires `${WALLET_MNEMONIC}`, `${ALTCHA_SECRET}`, and `${ALTCHA_KEY_SECRET}`; the wallet passphrase is omitted and defaults empty.

For a secret-free baseline use `config/minimal.toml`, which omits wallets. For full-demo use, inject a separately generated protected mnemonic and independent ALTCHA secrets through your runtime secret mechanism; restrict CORS origins before exposure.
