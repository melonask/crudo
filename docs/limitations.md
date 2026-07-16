# Limitations and troubleshooting

## Known boundaries

| Boundary | Detail |
|---|---|
| Trusted configuration | SQL is configured text and explicitly marked safe internally; configuration authors are trusted. |
| Process-local state | Request rate and ALTCHA replay state are in-memory per process. See [deployment](/operations/deployment). |
| Database drivers | Only SQLite and PostgreSQL SQLx drivers are configured. |
| Wallet support | Limited to the three profile combinations in the [wallet reference](/reference/wallets). |
| Wallet storage | Public address and path metadata are stored, not private keys. Protect root secrets and plan profile-name migrations. |

## Troubleshooting

### Startup says an environment variable is not set

**Cause:** The selected TOML contains `${NAME}` without a supplied value.

**Fix:** Provide the value or remove the optional feature table.

Environment requirements by configuration:

- Built-in minimal starter: none
- Full wallet demos: `WALLET_MNEMONIC`, `ALTCHA_SECRET`, and `ALTCHA_KEY_SECRET`

`WALLET_MNEMONIC` is not globally required by crudo.

### No endpoints configured, unknown action, or duplicate route

**Cause:** A configuration needs at least one endpoint, every endpoint must name an existing action, and method/path pairs must be unique after prefixing.

**Fix:** Add an endpoint, correct its action name, or make the resulting method/path pair unique.

### PostgreSQL type error

**Cause:** Path and query parameters are strings, but the SQL comparison expects a numeric type.

**Fix:** Cast the value in SQL, for example `$1::BIGINT`. Use numbered PostgreSQL parameters rather than SQLite `?` placeholders.

### `429` or ALTCHA failures behind a proxy

**Cause:** Crudo uses the direct proxy peer as the IP.

**Fix:** Enforce public-IP policy at the proxy. Account for sticky routing, or disable IP binding only after assessing replay risk.

### Wallet action fails

**Cause:** The mnemonic, profile set, profile combination, placeholder values, or primary result is invalid.

**Fix:** Confirm a valid mnemonic, nonempty profile set, compatible curve/derivation/format, exact placeholder values below `2^31`, and a `one` primary result.
