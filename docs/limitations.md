# Limitations and troubleshooting

## Known boundaries

- SQL is configured text and is explicitly marked safe internally: configuration authors are trusted.
- Request rate and ALTCHA replay state are in-memory per process; see [deployment](/operations/deployment).
- Only SQLite and PostgreSQL are configured SQLx drivers. Wallet support is limited to the three profile combinations in the [reference](/reference/wallets).
- Wallet derivation stores public address/path metadata, not private keys. Protect root secrets and plan profile-name migrations.

## Troubleshooting

**Startup says an environment variable is not set.** Locate `${NAME}` in the selected TOML and provide it, or remove the optional feature table. The minimal config needs none. `WALLET_MNEMONIC` is required by full wallet demo files, not by crudo globally; those files also require `ALTCHA_SECRET` and `ALTCHA_KEY_SECRET`.

**No endpoints configured / unknown action / duplicate route.** Every configuration needs at least one endpoint, each endpoint names an existing action, and method/path pairs must be unique after prefixing.

**PostgreSQL type error.** Path/query parameters are strings; cast them in SQL (`$1::BIGINT`). Use numbered PostgreSQL parameters, not SQLite `?` conventions.

**429 or ALTCHA failures behind a proxy.** The direct proxy peer is used as the IP. Enforce public-IP policy at the proxy and account for sticky routing or disable IP binding only after assessing replay risk.

**Wallet action fails.** Confirm valid mnemonic, a nonempty profile set, compatible curve/derivation/format, exact placeholder values, values below `2^31`, and a `one` primary result.
