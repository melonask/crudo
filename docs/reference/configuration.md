# Configuration reference

All fields are TOML fields. Unlisted defaults are not inferred.

::: danger Strict schema validation
All static configuration tables reject unknown fields. A misspelled protection, limit, endpoint, action, authentication, ALTCHA, or wallet field fails startup rather than silently using a default.

Dynamic action names and wallet `values` keys remain user-defined map keys.
:::

## Root and database

| Field | Required | Default | Validation / behavior |
|---|---|---|---|
| `database.url` | Yes | — | SQLx SQLite or PostgreSQL connection URL. |
| `database.setup` | No | `[]` | Statements run atomically before serving. |

## Server and CORS

| Field | Required | Default | Validation / behavior |
|---|---|---|---|
| `server.address` | No | `127.0.0.1:3000` | Listen address; CLI may override it. |
| `server.prefix` | No | `""` | Mounted before endpoint and ALTCHA paths. Configured paths must start with `/`. |
| `server.cors.origins` | No | — | When `[server.cors]` exists, this nonempty string array is required. Allows all methods and `Authorization`/`Content-Type`. |

## Limits

Server limits apply by default. `[endpoints.limits]` accepts optional versions of every field below and overrides the server value.

| Field | Required | Default | Validation / behavior |
|---|---|---|---|
| `server.limits.body_bytes` | No | `1048576` | `usize`; must be greater than `0`. |
| `server.limits.timeout_seconds` | No | `30` | `u64`; must be greater than `0`. |
| `server.limits.concurrency` | No | `100` | `usize`; must be greater than `0`. |
| `server.limits.requests` | No | `120` | `u32`; per direct-IP, endpoint-local window. `0` disables limiting. |
| `server.limits.window_seconds` | No | `60` | `u64`; must be greater than `0` when requests are enabled. |

## Endpoints

| Field | Required | Default | Validation / behavior |
|---|---|---|---|
| `endpoints.method` | Yes | — | One of GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS, or TRACE. |
| `endpoints.path` | Yes | — | Must start with `/`. |
| `endpoints.action` | Yes | — | Must name an existing action. |
| `endpoints.auth` | No | `[]` | Contains `basic` and/or `bearer`; each needs its matching auth table. |
| `endpoints.auth_optional` | No | `false` | Valid only when `auth` is configured. |
| `endpoints.altcha` | No | `false` | Requires `[altcha]` when true. |
| `endpoints.altcha_for_authenticated` | No | `true` | May be false only when `altcha` and `auth` are configured. |
| `endpoints.limits` | No | — | Optional overrides for all five limit fields. |

- Duplicate method/path pairs fail startup.
- Endpoint paths and the ALTCHA challenge path are mounted under `server.prefix`.

## Actions

| Field | Required | Default | Validation / behavior |
|---|---|---|---|
| `actions.NAME.sql` | Yes | — | SQL configured for the action. |
| `actions.NAME.params` | No | `[]` | Bound in declared order. |
| `actions.NAME.hash` | No | `[]` | Named request fields to Argon2-hash. |
| `actions.NAME.result` | No | `execute` | `execute`, `one`, `optional`, or `many`. |
| `actions.NAME.status` | No | `200` | Must be a valid 2xx status at startup. |
| `actions.NAME.no_store` | No | `false` | Adds `Cache-Control: no-store` when true. |
| `actions.NAME.errors` | No | `[]` | Configured database-message mappings. |

Each `[[actions.NAME.errors]]` entry requires `database_message`, a 400–599 `status`, and response `message`.

## Authentication

Authentication tables are optional until an endpoint names their method.

| Field | Required | Default | Validation / behavior |
|---|---|---|---|
| `auth.basic.sql` | Yes for Basic | — | SQL that selects the owner and password hash. |
| `auth.basic.owner` | Yes for Basic | — | Selected owner-column name. |
| `auth.basic.password` | Yes for Basic | — | Selected password-hash column name. |
| `auth.bearer.sql` | Yes for Bearer | — | SQL that resolves the owner. |
| `auth.bearer.owner` | Yes for Bearer | — | Selected owner-column name. |

## ALTCHA

| Field | Required | Default | Validation / behavior |
|---|---|---|---|
| `altcha.secret` | Yes when `[altcha]` exists | — | ALTCHA secret. |
| `altcha.key_secret` | Yes when `[altcha]` exists | — | Independent ALTCHA key secret. |
| `altcha.path` | No | `/challenge` | Challenge route path. |
| `altcha.algorithm` | No | `PBKDF2/SHA-256` | Challenge algorithm. |
| `altcha.cost` | No | `5000` | Challenge cost. |
| `altcha.max_number` | No | `10000` | Challenge maximum number. |
| `altcha.expires_seconds` | No | `300` | Challenge lifetime. |
| `altcha.bind_ip` | No | `true` | Binds proofs to the direct peer IP. |

- `[altcha]` is required when an endpoint sets `altcha = true`.
- A challenge GET route is mounted whenever `[altcha]` exists.
- The challenge route must not conflict with a configured GET route.

## Wallets

| Field | Required | Default | Validation / behavior |
|---|---|---|---|
| `wallets.mnemonic` | Yes when `[wallets]` exists | — | BIP-39 mnemonic source. |
| `wallets.passphrase` | No | `""` | Optional passphrase. |
| `wallets.profiles` | Yes when `[wallets]` exists | `[]` | Must be nonempty. |
| `wallets.profiles.name` | Yes | — | Profile name. |
| `wallets.profiles.caip2` | Yes | — | CAIP-2 identifier. |
| `wallets.profiles.curve` | Yes | — | Supported curve. |
| `wallets.profiles.derivation` | Yes | — | Supported derivation method. |
| `wallets.profiles.path` | Yes | — | Derivation path template. |
| `wallets.profiles.address_format` | Yes | — | Supported address format. |
| `wallets.profiles.network` | Conditional | — | Required only for `p2wpkh`. |
| `wallets.profiles.max_addresses` | Yes | — | Maximum addresses for the profile. |

See [wallets](./wallets) for accepted combinations and path validation.

## Wallet action stages

| Field | Required | Default | Validation / behavior |
|---|---|---|---|
| `actions.NAME.wallets.sql` | Yes | — | Address persistence SQL. |
| `actions.NAME.wallets.params` | Yes | — | Persistence parameters. |
| `actions.NAME.wallets.values` | Yes | — | Placeholder values for selected profile paths. |
| `actions.NAME.wallets.profiles` | Exactly one selector | — | Nonempty configured profile names. |
| `actions.NAME.wallets.profile` | Exactly one selector | — | Request input field naming a configured profile. |

- A wallet stage requires `[wallets]` and a parent `result = "one"` action.
- `profiles` and `profile` are mutually exclusive.
- `values` maps every `{placeholder}` in each selected profile path exactly once to a decimal `u32` or `$result.column`.
