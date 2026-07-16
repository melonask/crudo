# Configuration reference

All fields below are TOML fields. Unlisted defaults are not inferred.

## Root and server

| Field | Type / default | Rule |
|---|---|---|
| `database.url` | string, required | SQLx SQLite or PostgreSQL connection URL. |
| `database.setup` | string array, `[]` | Statements run atomically before serving. |
| `server.address` | string, `127.0.0.1:3000` | Listen address; CLI may override it. |
| `server.prefix` | string, `""` | Mounted before endpoint and ALTCHA paths; configured paths must start `/`. |
| `server.cors.origins` | string array | Optional CORS table; when present it must be nonempty. Allows all methods and `Authorization`/`Content-Type`. |
| `server.limits.body_bytes` | usize, `1048576` | Must be > 0. |
| `.timeout_seconds` | u64, `30` | Must be > 0. |
| `.concurrency` | usize, `100` | Must be > 0. |
| `.requests` | u32, `120` | Per direct-IP, endpoint-local window; `0` disables limiting. |
| `.window_seconds` | u64, `60` | Must be > 0 when requests is enabled. |

`[[endpoints]]` requires `method`, `path`, and existing `action`. Supported methods are GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS, TRACE. Duplicate method/path pairs fail startup. `auth` is `[]` by default and contains `basic` and/or `bearer`; each requires its corresponding auth table. `auth_optional` defaults false and is valid only with `auth`. `altcha` defaults false; `altcha_for_authenticated` defaults true and can be false only with `altcha` and auth. `[endpoints.limits]` has optional versions of all five limit fields and overrides server values.

## Actions, authentication, ALTCHA

`[actions.NAME]` requires `sql`; `params`, `hash`, and `errors` default empty; `result` defaults `execute`; `no_store` defaults false; `status` is optional and, when supplied, must be a valid 2xx HTTP status at startup. `result` is `execute`, `one`, `optional`, or `many`. Each `[[actions.NAME.errors]]` requires `database_message`, 400–599 `status`, and response `message`.

All static configuration tables reject unknown fields. A misspelled protection, limit, endpoint, action, authentication, ALTCHA, or wallet field therefore fails startup instead of silently falling back to a default. Dynamic action names and wallet `values` keys remain user-defined map keys.

`[auth.basic]` requires SQL `sql`, selected owner-column `owner`, and password-hash column `password`. `[auth.bearer]` requires `sql` and `owner`. Both tables are optional unless a route names their method.

`[altcha]` is optional unless an endpoint sets `altcha = true`. It requires `secret` and `key_secret`; `path` defaults `/challenge`, `algorithm` `PBKDF2/SHA-256`, `cost` `5000`, `max_number` `10000`, `expires_seconds` `300`, and `bind_ip` `true`. An ALTCHA challenge route is mounted whenever `[altcha]` exists, and it must not conflict with a configured GET route.

## Wallet schema

`[wallets]` is optional, but requires `mnemonic` when present; `passphrase` is string/default `""`; `profiles` defaults `[]` but must be nonempty. A profile requires `name`, `caip2`, `curve`, `derivation`, `path`, `address_format`, and `max_addresses`; `network` is optional and only required for `p2wpkh`. See [wallets](./wallets) for allowed values and validation.

`[actions.NAME.wallets]` requires `sql`, `params`, `values`, and exactly one of `profiles` (nonempty names) or `profile` (input field name). The parent must be `result = "one"`, and `[wallets]` must exist. `values` exactly maps every `{placeholder}` in each selected profile path to a decimal u32 or `$result.column`.
