# Configuration workflow

Configuration is TOML. `${NAME}` is expanded everywhere before TOML parsing, with no escaping or fallback syntax. Empty names, unclosed forms, and unset variables abort startup. Thus `${WALLET_MNEMONIC}` is mandatory only when it appears in the selected file.

`[wallets]` itself is optional. To run without a mnemonic, omit `[wallets]` and every `actions.<name>.wallets` stage. `passphrase` is optional and defaults to an empty string; the shipped full configurations omit `WALLET_PASSPHRASE`. Their wallet stages make `${WALLET_MNEMONIC}` required. Their ALTCHA configuration also makes `${ALTCHA_SECRET}` and `${ALTCHA_KEY_SECRET}` required.

Load a remote configuration only over HTTPS you control. It is fetched at start and its expanded secrets live in process memory; protect its transport, source, and deployment environment.
