# Wallets and profiles

::: info Wallets are optional
`[wallets]` is not globally required. It is required only for a referenced wallet action, so non-wallet APIs should omit both `[wallets]` and wallet stages.

`${WALLET_MNEMONIC}` makes that variable required only in configurations that reference it. `passphrase` defaults to an empty string. The shipped store configurations do not configure wallets.
:::

Wallets derive public addresses from a BIP-39 mnemonic and optional passphrase. Private keys are not stored.

## Profile fields

| Field | Rule |
|---|---|
| `name` | Unique and whitespace-free. |
| `caip2` | `namespace:reference`. The namespace is 3–8 lowercase letters, digits, or hyphens; the reference is 1–32 alphanumeric characters, hyphens, or underscores. |
| `curve` | Must match a supported combination. |
| `derivation` | Must match a supported combination. |
| `path` | A valid path template with unique placeholders. |
| `address_format` | Must match a supported combination. |
| `network` | Required only for `p2wpkh`. |
| `max_addresses` | Integer from `1` through `2^31`. |

## Supported combinations

| Curve | Derivation | Address format | Output / network |
|---|---|---|---|
| `secp256k1` | `bip32` | `evm` | EIP-55 address. No `network`. |
| `ed25519` | `slip10` | `base58-public-key` | Solana-style public key. No `network`. |
| `secp256k1` | `bip32` | `p2wpkh` | Native SegWit Bitcoin address. `network` is `mainnet`, `testnet`, `signet`, or `regtest`. |

## Derivation and path rules

- Paths contain unique `{name}` placeholders.
- `values` must match those placeholders exactly, and every value must be below `2^31`.
- SLIP-10 paths start with `m/`, contain children, and every child ends with `'`.
- A stage with `profiles` derives every named configured profile.
- A stage with `profile = "field"` validates that request field against configured profiles.

## Wallet-stage transaction

A wallet stage follows a successful `one` primary action. Primary SQL, derivation, and all persistence inserts run in one database transaction. Any failure rolls the work back.

See [actions](./actions#wallet-transaction-stage) for accepted persistence references and [configuration](./configuration#wallet-action-stages) for stage fields.

## Security boundaries

- Only public address and derivation-path metadata are persisted.
- Protect the mnemonic and optional passphrase as root secrets.
- Plan profile-name migrations before changing a deployed profile name.
