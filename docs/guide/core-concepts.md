# Core concepts

## Endpoints and actions

An **endpoint** maps an HTTP method and path to a named **action**. The action executes configured SQL, binds its `params` in order, and serializes its result as JSON.

There is no implicit CRUD. Every route and query is deliberate.

## Request input

| Priority | Source | Duplicate-name behavior |
|---|---|---|
| 1 | Query parameters | Initial value; strings. |
| 2 | Path parameters | Replaces query value; strings. |
| 3 | JSON-object body | Replaces path/query value. |

The body must be a JSON object.

Crudo can also add values that do not come from the request:

- Successful authentication adds `$owner`.
- Listing `$token` in `params` generates a fresh random token value.

## Transformations and wallet stages

- Actions can Argon2-hash named string fields with `hash` before binding.
- A wallet stage first runs a required `one` primary action.
- It derives addresses from returned columns, persists them, and commits all work together.
- Any wallet-stage failure rolls the transaction back.

## JSON row conversion

| Database value | JSON representation |
|---|---|
| Integer, float, boolean | Matching JSON scalar |
| Blob / `BYTEA` | Base64 string |
| `NULL` | `null` |
| Other type | String |
