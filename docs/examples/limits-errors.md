# Limits and mapped errors

Server limits establish defaults; an endpoint can override any of them. This configuration limits every route, makes writes stricter, and converts a deliberate SQLite validation failure into a useful `422`. It omits `server.prefix`, so its route is `/transfers`.

```toml
[server.limits]
body_bytes = 65536
timeout_seconds = 15
concurrency = 32
requests = 120
window_seconds = 60

[database]
url = "sqlite://limits.db?mode=rwc"
setup = [
  "CREATE TABLE IF NOT EXISTS transfers (id INTEGER PRIMARY KEY AUTOINCREMENT, amount INTEGER NOT NULL)",
  "CREATE TRIGGER IF NOT EXISTS transfers_positive BEFORE INSERT ON transfers WHEN NEW.amount <= 0 BEGIN SELECT RAISE(ABORT, 'amount must be positive'); END"
]

[[endpoints]]
method = "POST"
path = "/transfers"
action = "create_transfer"

[endpoints.limits]
requests = 3
window_seconds = 60
body_bytes = 1024

[actions.create_transfer]
sql = "INSERT INTO transfers (amount) VALUES (?) RETURNING id, amount"
params = ["amount"]
result = "one"
status = 201

[[actions.create_transfer.errors]]
database_message = "amount must be positive"
status = 422
message = "amount must be positive"
```

```sh
crudo --config ./limits.toml
curl -i -X POST http://127.0.0.1:3000/transfers -H 'content-type: application/json' -d '{"amount":0}'
curl -i -X POST http://127.0.0.1:3000/transfers -H 'content-type: application/json' -d '{"amount":5}'
```

The invalid amount returns `422 {"error":"amount must be positive"}`. After three requests from one direct IP in 60 seconds, the endpoint returns `429` with a `Retry-After` header in seconds; wait that long before retrying. The limit is endpoint-local and process-local, so apply public-client rate limiting at a trusted proxy for deployed or multi-replica services.

::: warning Handle layer responses by status
The body-size and timeout layers may return `413` and `408` before an action runs. Those layer-generated responses are not guaranteed to use crudo's JSON error shape; clients must not assume JSON for them.
:::
