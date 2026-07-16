# HTTP behavior

Application-generated successes and errors use JSON. The default success status is `200`.

## Request composition and precedence

Crudo merges input in this order: query parameters, path parameters, then the JSON body. Later sources win when names overlap.

| Source | Notes |
|---|---|
| Query | Values are strings. |
| Path | Values are strings and override query values. |
| Body | Must be a JSON object and overrides path/query values. |

## Content requirements

When supplied, a request body must be a JSON object.

```json
{"error":"body must be a JSON object"}
```

This response has status `400`. A missing required action parameter also returns `400`.

## Success behavior

Action result modes select the JSON shape. An action may configure a different 2xx `status`, and `no_store = true` adds `Cache-Control: no-store`.

See the [actions reference](./actions#result-modes) for the result-mode response shapes.

## Error statuses

| Status | Meaning |
|---|---|
| 400 | Invalid JSON shape, missing parameter, invalid hash/profile input, or out-of-range integer. |
| 401 | Missing or invalid Basic/Bearer credentials. |
| 403 | Missing, invalid, expired, reused, or IP-mismatched ALTCHA proof. |
| 404 | A `one` result found no row. |
| 409 | Unique or foreign-key database constraint conflict. |
| 422 | A configured action error, such as insufficient balance. |
| 429 | Per-IP limit exceeded; includes `Retry-After` seconds. |
| 500 | Unmapped database, derivation, or server error. |

For example, invalid credentials return:

```json
{"error":"invalid or missing credentials"}
```

## Cache headers

| Response | Header |
|---|---|
| ALTCHA challenge GET | `Cache-Control: no-store` |
| Action with `no_store = true` | `Cache-Control: no-store` |

Challenge verification expects a Base64-encoded ALTCHA payload in the body field named `altcha`. Proofs are single-use within one process.

::: warning Layer-generated responses may not be JSON
The body-size and timeout layers can reject a request, commonly with `413` and `408`. Their body and media type are layer-generated, so clients must handle them by status and headers rather than assuming the JSON error shape.
:::
