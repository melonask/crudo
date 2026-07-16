# Actions reference

## Result, parameters, and response

`execute` returns `{"rows_affected": n}`; `one` fetches exactly one row (`404 {"error":"resource not found"}` if absent); `optional` returns a row or `null`; `many` returns an array. A configured success `status` replaces 200. `no_store = true` adds `Cache-Control: no-store`.

Values are bound in `params` order. Normal names come from merged request input. `$owner` is injected only after successful auth and is the configured SQL owner value. `$token` is generated only if it appears in action params. Named `hash` fields must be strings and are replaced by Argon2 hashes. Missing params and non-string hash/profile fields are `400`.

For wallet persistence params, only `$result.<column>`, `$profile.name`, `$profile.caip2`, `$profile.max_addresses`, `$wallet.address`, and `$wallet.derivation_path` are accepted. Result references must exist. Wallet path values must be u32 below `2^31`.

Expected database messages can be mapped with `errors`; unmapped unique/foreign-key violations become `409`, absent `one` rows become `404`, and other database/internal failures become `500` without detail. A configured action success status must be 2xx and is validated at startup; error-map statuses are likewise validated at startup.

```toml
[actions.create_account]
sql = "INSERT INTO accounts (email, password) VALUES (?, ?) RETURNING id, email"
params = ["email", "password"]
hash = ["password"]
result = "one"
status = 201
```
