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

### `crudo` did not start the API I expected

**Cause:** `--config` selects a local path or HTTPS URL; otherwise crudo reads `./Crudo.toml`. There are no embedded routes or fallback configuration.

**Fix:** Run with the intended explicit `--config` value, or create/update `./Crudo.toml`. Missing, malformed, or unreadable selected configuration is an error.

### Startup reports a database or table error

**Cause:** Omitting `[database]` only defaults to `sqlite://crudo.db?mode=rwc` and an empty setup list. It does not create tables required by custom SQL.

**Fix:** Add idempotent `database.setup` statements for a small service, or apply managed migrations before startup in production.

### Cargo mentions `generic-array`, `matchit`, or an “available” dependency

`generic-array` 0.14.7 is transitive through RustCrypto `digest`/`crypto-common`; `crypto-common` 0.1.7 pins `generic-array = "=0.14.7"`. `matchit` 0.8.4 is transitive through Axum routing; resolved Axum 0.8.x pins `matchit = "=0.8.4"`. `(available: newer)` only reports a newer registry release that current transitive constraints disallow—not a stale direct crudo dependency or an installation failure.

### Startup says an environment variable is not set

**Cause:** The selected TOML contains `${NAME}` without a supplied value.

**Fix:** Provide the value or remove the optional feature table.

Environment requirements by configuration:

- A configuration without environment expansions: none
- `config/sqlite.toml`: none
- `config/postgres.toml`: `DATABASE_URL`
- Wallet or ALTCHA variables: only when the selected configuration references those optional features

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
