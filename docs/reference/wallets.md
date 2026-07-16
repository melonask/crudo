# Wallets and profiles

Wallets derive public addresses from a BIP-39 mnemonic and optional passphrase; private keys are not stored. `[wallets]` is not globally required. It becomes required only for a referenced wallet action. Therefore omit it and wallet stages for non-wallet APIs. `${WALLET_MNEMONIC}` makes that environment variable mandatory in a full wallet-demo config. `passphrase` defaults empty, and the shipped configurations omit `WALLET_PASSPHRASE`.

Profiles have unique, whitespace-free names; CAIP-2 must be `namespace:reference` with a 3–8 lowercase/digit/hyphen namespace and 1–32 alphanumeric/hyphen/underscore reference. `max_addresses` is 1 through `2^31`.

| Curve + derivation + format | Output |
|---|---|
| `secp256k1` + `bip32` + `evm` | EIP-55 address; no `network`. |
| `ed25519` + `slip10` + `base58-public-key` | Solana-style public key; every path child hardened; no `network`. |
| `secp256k1` + `bip32` + `p2wpkh` | Native SegWit Bitcoin address; `network` required: `mainnet`, `testnet`, `signet`, or `regtest`. |

Paths must contain unique `{name}` placeholders. Values must match exactly and be below `2^31`; SLIP-10 paths start `m/`, contain children, and every child ends `'`. A stage with `profiles` derives each configured profile; one with `profile = "field"` validates that request field against configured profiles. Primary SQL, derivation, and every persistence insert run in one database transaction.
