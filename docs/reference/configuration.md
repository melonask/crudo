# Configuration reference

All fields are TOML fields. Unlisted defaults are not inferred.

## Configuration selection

The CLI uses `--config` to select a local path or HTTPS URL. Without it, it reads `./Crudo.toml`. If neither is available, startup fails with guidance. An unreadable or malformed selected configuration also fails startup. Installed binaries do not require the source-tree `config/store.toml` bootstrap.

::: danger Strict schema validation
All static configuration tables reject unknown fields. A misspelled protection, limit, endpoint, action, authentication, ALTCHA, or wallet field fails startup rather than silently using a default.

Dynamic action names and wallet `values` keys remain user-defined map keys.
:::

## Root and database

| Field | Required | Default | Validation / behavior |
|---|---|---|---|
| `database.url` | No | `sqlite://crudo.db?mode=rwc` | Only `sqlite:` or `postgres:`/`postgresql:` connection URLs are accepted. |
| `database.setup.sqlite` | No | empty | SQLite-only setup statements and sources. |
| `database.setup.postgres` | No | empty | PostgreSQL-only setup statements and sources. |
| `database.setup.common` | No | empty | Setup statements and sources for both backends. |

Omitting `[database]` selects the local SQLite URL above and runs no setup statements. It does not create application tables; define the applicable `database.setup.sqlite`, `database.setup.postgres`, or `database.setup.common` table, or manage the schema separately when routes need tables.

### Database setup sources

Setup is strictly partitioned into `sqlite`, `postgres`, and `common` tables. For the selected URL, Crudo loads the selected backend table first and `common` second. Within each table, inline `statements` run first, then `sources` in listed order. It loads and validates both tables and every source before opening one transaction; all resulting statements execute in that order in that one transaction. Any failure rolls it back.

```toml
[database]
url = "sqlite://app.db?mode=rwc"

[database.setup.sqlite]
statements = ["CREATE TABLE entries (id INTEGER PRIMARY KEY, body TEXT NOT NULL)"]
sources = [
  { location = "schema/seed.sql", format = "sql" },
  { location = "https://config.example.test/entries.json", format = "json" },
]
```

`location` is a local text-file path or an HTTPS URL; plain HTTP is rejected. A `sql` source must be nonempty and is executed as one raw SQL batch. A `json` source must be either an array of nonempty SQL strings or a strict object containing only `{"statements":[...]}`. Configuration and every loaded source expand `${NAME}` environment variables; an unset, empty-name, or unclosed reference fails startup.

## Server and CORS

| Field | Required | Default | Validation / behavior |
|---|---|---|---|
| `server.address` | No | `127.0.0.1:3000` | Listen address; CLI may override it. |
| `server.prefix` | No | `""` | Mounted before endpoint and ALTCHA paths. Configured paths must start with `/`. |
| `server.cors.origins` | No | — | When `[server.cors]` exists, this nonempty string array is required. Allows all methods and `Authorization`/`Content-Type`. |

Configure only exact origins for the deployment. Configuration strings support `${ENV}` expansion, including CORS origins, so deployment-specific values do not require Rust changes.

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
| `endpoints.roles` | No | `[]` | Nonempty, unique allowed roles; requires mandatory authentication and a role column for every configured method. |
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
| `actions.NAME.boolean_columns` | No | `[]` | Result-column names normalized to JSON booleans; names must be nonempty and unique. |
| `actions.NAME.result` | No | `execute` | `execute`, `one`, `optional`, or `many`. |
| `actions.NAME.status` | No | `200` | Must be a valid 2xx status at startup. |
| `actions.NAME.no_store` | No | `false` | Adds `Cache-Control: no-store` when true. |
| `actions.NAME.errors` | No | `[]` | Configured database-message mappings. |

Each `[[actions.NAME.errors]]` entry requires `database_message`, a 400–599 `status`, and response `message`.

For each configured `boolean_columns` name, every returned row must contain that column. Crudo serializes a native boolean or integer `0`/`1` as a JSON boolean; a missing column or any other value fails the action. Use this when SQLite and PostgreSQL return equivalent boolean values with different native representations.

### Backend SQL

Every bound SQL field accepts either one nonempty universal string or an object with both nonempty variants:

```toml
sql = "SELECT id FROM entries WHERE id = $1"
# or
sql = { sqlite = "SELECT id FROM entries WHERE id = $1", postgres = "SELECT id FROM entries WHERE id = $1::BIGINT" }
```

Universal bound SQL uses `$1`, `$2`, and so on for both SQLite and PostgreSQL. Crudo does not translate SQL at runtime; use variants when syntax differs.

### x402 payment-required errors

An error mapping may add an x402 lookup with `[actions.NAME.errors.x402]` (immediately after the relevant `[[actions.NAME.errors]]` entry):

```toml
[[actions.purchase.errors]]
database_message = "insufficient balance"
status = 402
message = "insufficient balance"

[actions.purchase.errors.x402]
sql = "SELECT payment_required FROM payment_requirements WHERE product_id = $1"
params = ["product_id"]
column = "payment_required"
```

`x402` is valid only on a `402` mapping. Its SQL receives named input values in `params` order and must return one row whose named `column` is textual JSON for a canonical x402 v2 `PaymentRequired` payload. Crudo validates `x402Version: 2`, an object `resource`, an `accepts` array whose entries contain textual `scheme`, `network`, `amount`, `asset`, and `payTo` plus nonnegative integer `maxTimeoutSeconds`. If present, `extensions` must be an object and every extension must contain object-valued `info` and `schema`.

On a valid lookup, Crudo returns that payload as the `402` JSON body and Base64-encodes the same JSON in `PAYMENT-REQUIRED`. It does not process payment. A missing parameter, query/row/column/type failure, invalid JSON, or invalid payload is an x402 construction failure and becomes a generic `500` response.

## Authentication

Authentication tables are optional until an endpoint names their method.

| Field | Required | Default | Validation / behavior |
|---|---|---|---|
| `auth.basic.sql` | Yes for Basic | — | SQL that selects the owner and password hash. |
| `auth.basic.owner` | Yes for Basic | — | Selected owner-column name. |
| `auth.basic.password` | Yes for Basic | — | Selected password-hash column name. |
| `auth.basic.role` | Required when an endpoint using Basic sets `roles` | — | Selected role-column name. |
| `auth.bearer.sql` | Yes for Bearer | — | SQL that resolves the owner. |
| `auth.bearer.owner` | Yes for Bearer | — | Selected owner-column name. |
| `auth.bearer.role` | Required when an endpoint using Bearer sets `roles` | — | Selected role-column name. |

`roles` cannot be used with `auth_optional = true`. A successfully authenticated caller whose selected role is not allowed receives `403`.

Configured `owner`, `password`, and `role` column names must not be blank.

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
