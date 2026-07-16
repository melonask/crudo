# Actions reference

An action executes configured SQL with values bound in its declared `params` order. Its `result` mode determines the JSON response.

## Execution flow

1. Crudo merges request input and resolves special parameters.
2. It hashes fields named by `hash` before binding them.
3. It executes the primary SQL action.
4. If configured, it derives and persists wallet addresses in the same transaction.
5. It serializes the selected result mode or maps the failure to an error response.

## Input precedence

Request values merge in this order. Later sources replace duplicate names.

| Order | Source | Value type |
|---|---|---|
| 1 | Query parameters | Strings |
| 2 | Path parameters | Strings |
| 3 | JSON-object body | JSON values |

The body, when present, must be a JSON object. Missing parameters produce `400`.

## Special parameters

| Parameter | Source | Rule |
|---|---|---|
| `$owner` | Successful authentication | The configured SQL owner value. |
| `$token` | Crudo | A fresh random Argon2 salt string, generated only when listed in `params`. |
| Named `hash` field | Request input | Must be a string; replaced with an Argon2 hash. |

Non-string hash or profile fields produce `400`.

## Hashing

Use `hash` for request fields that must be stored as Argon2 hashes. The original named string is replaced before SQL binding.

```toml
[actions.create_account]
sql = "INSERT INTO accounts (email, password) VALUES (?, ?) RETURNING id, email"
params = ["email", "password"]
hash = ["password"]
result = "one"
status = 201
```

This action returns the inserted account without returning the password hash.

## Result modes

| Mode | Successful response | Missing row behavior |
|---|---|---|
| `execute` | `{"rows_affected": n}` | Not applicable |
| `one` | Exactly one row object | `404 {"error":"resource not found"}` |
| `optional` | One row object or `null` | `null` with success status |
| `many` | Array of row objects | Empty array |

## Success controls

| Setting | Default | Effect | Validation |
|---|---|---|---|
| `status` | `200` | Replaces the default success status. | Must be a 2xx status at startup. |
| `no_store` | `false` | Adds `Cache-Control: no-store`. | Boolean. |

## Wallet transaction stage

A wallet stage requires a primary action with `result = "one"`. The primary SQL, address derivation, and every persistence insert run in one database transaction.

For wallet persistence parameters, only these references are accepted:

- `$result.<column>`
- `$profile.name`, `$profile.caip2`, and `$profile.max_addresses`
- `$wallet.address` and `$wallet.derivation_path`

Each `$result.<column>` reference must exist. Wallet path values must be `u32` values below `2^31`.

## Mapped and default errors

| Condition | Response |
|---|---|
| Matching configured `errors` entry | Configured 400–599 status and message |
| Unique or foreign-key violation without a mapping | `409` |
| `one` finds no row | `404` |
| Other database, derivation, or internal failure | `500` without detail |

Every `[[actions.NAME.errors]]` entry requires `database_message`, `status`, and `message`. Error-map statuses are validated at startup.

## SQLite and PostgreSQL parameters

SQLite uses positional `?` placeholders.

```toml
[actions.get_user]
sql = "SELECT id, email FROM users WHERE id = ?"
params = ["id"]
result = "one"
```

PostgreSQL uses numbered placeholders. Path and query values are strings, so compare them with numeric columns using an explicit cast.

```toml
[actions.get_user]
sql = "SELECT id, email FROM users WHERE id = $1::BIGINT"
params = ["id"]
result = "one"
```

Both actions bind the incoming `id`; the PostgreSQL form converts it for a `BIGINT` comparison.
