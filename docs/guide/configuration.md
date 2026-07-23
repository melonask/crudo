# Configuration workflow

## Select configuration deliberately

`--config` selects a local path or HTTPS URL explicitly. Without it, crudo reads `./Crudo.toml`. If neither is available, startup fails with guidance; unreadable or invalid selected configuration also fails startup. Installed binaries do not depend on a repository-relative `config/sqlite.toml`.

For production, use an explicit reviewed `--config` path or URL, declare the database URL, and manage schema changes deliberately. Omitting `[database]` uses local `sqlite://crudo.db?mode=rwc` with no setup statements; it does not create custom tables. Omitting `[server]` uses `127.0.0.1:3000` with no prefix; set `prefix = "v1"` explicitly to mount routes below `/v1`.

## Expand environment variables

Configuration is TOML. `${NAME}` is expanded everywhere before TOML parsing.

| Form | Behavior |
|---|---|
| `${NAME}` | Replaced with the environment value. |
| Empty name, unclosed form, or unset variable | Startup fails. |

There is no escaping or fallback syntax.

## Choose the features you need

- `[wallets]` is optional.
- To run without a mnemonic, omit `[wallets]` and every `actions.<name>.wallets` stage.
- `passphrase` defaults to an empty string.
- The shipped store configurations configure wallets and ALTCHA.

::: info Conditional feature secrets
A configuration with wallet stages may require `${WALLET_MNEMONIC}`; one with `[altcha]` requires its configured secrets. Neither is globally required by crudo. Both shipped store configurations require `${WALLET_MNEMONIC}`, `${ALTCHA_SECRET}`, and `${ALTCHA_KEY_SECRET}`; `config/postgres.toml` also requires `${DATABASE_URL}`.
:::

## Load remote configuration carefully

Remote configuration must use HTTPS you control. It is fetched at startup and its expanded secrets live in process memory.

Protect the transport, source, and deployment environment. See the complete [configuration reference](/reference/configuration).
