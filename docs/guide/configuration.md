# Configuration workflow

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
- The shipped full configurations omit `WALLET_PASSPHRASE`.

::: info Conditional full-demo secrets
Their wallet stages require `${WALLET_MNEMONIC}`. Their ALTCHA configuration requires `${ALTCHA_SECRET}` and `${ALTCHA_KEY_SECRET}`. `WALLET_MNEMONIC` is not globally required by crudo.
:::

## Load remote configuration carefully

Remote configuration must use HTTPS you control. It is fetched at startup and its expanded secrets live in process memory.

Protect the transport, source, and deployment environment. See the complete [configuration reference](/reference/configuration).
