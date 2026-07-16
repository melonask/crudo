# HTTP behavior

Every normal response is JSON. Request bodies, when supplied, must be JSON objects or return `400 {"error":"body must be a JSON object"}`. Successful default status is 200.

| Status | Meaning |
|---|---|
| 400 | Invalid JSON shape, missing parameter, invalid hash/profile input, or out-of-range integer. |
| 401 | Missing or invalid Basic/Bearer credentials: `invalid or missing credentials`. |
| 403 | Missing, invalid, expired, reused, or IP-mismatched ALTCHA proof. |
| 404 | `one` result found no row. |
| 409 | Unique/foreign-key database constraint conflict. |
| 422 | A configured action error, such as insufficient balance. |
| 429 | Per-IP limit exceeded; includes `Retry-After` seconds. |
| 500 | Unmapped database, derivation, or server error. |

ALTCHA challenge GET responses and `no_store` action responses have `Cache-Control: no-store`. Challenge verification expects a Base64-encoded ALTCHA payload in the body field named `altcha`; proofs are single-use in one process.

Application-generated successes and errors use JSON. Body-size and timeout layers can reject requests (commonly 413 and 408), but their response body and media type are layer-generated and not guaranteed to use the JSON error shape. Clients must handle those non-JSON responses by status and headers.
