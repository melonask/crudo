# Basic to Bearer authentication

This SQLite configuration hashes a password at registration, exchanges Basic credentials for a short-lived Bearer token, and scopes notes to the authenticated owner. Save it as `auth-notes.toml`. It omits `server.prefix`, so routes remain unprefixed.

```toml
[database]
url = "sqlite://auth-notes.db?mode=rwc"

[database.setup.sqlite]
statements = [
  "CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY AUTOINCREMENT, email TEXT NOT NULL UNIQUE, password TEXT NOT NULL, role TEXT NOT NULL DEFAULT 'customer')",
  "CREATE TABLE IF NOT EXISTS sessions (token TEXT PRIMARY KEY, user_id INTEGER NOT NULL, expires_at INTEGER NOT NULL, FOREIGN KEY (user_id) REFERENCES users(id))",
  "CREATE TABLE IF NOT EXISTS notes (id INTEGER PRIMARY KEY AUTOINCREMENT, user_id INTEGER NOT NULL, body TEXT NOT NULL, FOREIGN KEY (user_id) REFERENCES users(id))"
]

[auth.basic]
sql = "SELECT id, password, role FROM users WHERE email = $1"
owner = "id"
password = "password"
role = "role"

[auth.bearer]
sql = "SELECT s.user_id, u.role FROM sessions AS s JOIN users AS u ON u.id = s.user_id WHERE s.token = $1 AND s.expires_at > unixepoch()"
owner = "user_id"
role = "role"

[[endpoints]]
method = "POST"
path = "/users"
action = "create_user"

[[endpoints]]
method = "POST"
path = "/tokens"
action = "create_token"
auth = ["basic"]

[[endpoints]]
method = "POST"
path = "/notes"
action = "create_note"
auth = ["bearer"]
roles = ["customer"]

[[endpoints]]
method = "GET"
path = "/notes"
action = "list_notes"
auth = ["bearer"]

[actions.create_user]
sql = "INSERT INTO users (email, password) VALUES ($1, $2) RETURNING id, email"
params = ["email", "password"]
hash = ["password"]
result = "one"
status = 201

[actions.create_token]
sql = "INSERT INTO sessions (token, user_id, expires_at) VALUES ($1, $2, unixepoch() + 3600) RETURNING token, expires_at"
params = ["$token", "$owner"]
result = "one"
status = 201
no_store = true

[actions.create_note]
sql = "INSERT INTO notes (user_id, body) VALUES ($1, $2) RETURNING id, body"
params = ["$owner", "body"]
result = "one"
status = 201

[actions.list_notes]
sql = "SELECT id, body FROM notes WHERE user_id = $1 ORDER BY id"
params = ["$owner"]
result = "many"
```

## Exchange credentials

```sh
crudo --config ./auth-notes.toml
curl -X POST http://127.0.0.1:3000/users -H 'content-type: application/json' -d '{"email":"ada@example.test","password":"choose-a-long-unique-password"}'
curl -u 'ada@example.test:choose-a-long-unique-password' -X POST http://127.0.0.1:3000/tokens
# Copy token from the response, then:
curl -X POST http://127.0.0.1:3000/notes -H 'authorization: Bearer TOKEN' -H 'content-type: application/json' -d '{"body":"private note"}'
curl http://127.0.0.1:3000/notes -H 'authorization: Bearer TOKEN'
```

Example token and note responses:

```json
{"token":"generated-token","expires_at":1784235600}
{"id":1,"body":"private note"}
```

The token action sends `Cache-Control: no-store`. Tokens expire after one hour, and the Bearer SQL rejects expired sessions. The `role` columns select the authenticated role; `roles = ["customer"]` makes that endpoint mandatory-auth only and returns `403` to authenticated callers with another role. Use HTTPS, keep tokens out of logs and URLs, choose a production password policy, and add a server-side session-revocation policy if your application needs one.
