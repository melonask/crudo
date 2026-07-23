# Custom CRUD API

Save this complete SQLite configuration as `tasks.toml`. Its idempotent setup is safe to run each time the server starts. It omits `server.prefix`, so routes remain unprefixed.

```toml
[server]
address = "127.0.0.1:3000"

[server.limits]
body_bytes = 65536
timeout_seconds = 15
concurrency = 32
requests = 60
window_seconds = 60

[database]
url = "sqlite://tasks.db?mode=rwc"

[database.setup.sqlite]
statements = [
  "CREATE TABLE IF NOT EXISTS tasks (id INTEGER PRIMARY KEY AUTOINCREMENT, title TEXT NOT NULL, done INTEGER NOT NULL DEFAULT 0, created_at INTEGER NOT NULL DEFAULT (unixepoch()), updated_at INTEGER NOT NULL DEFAULT (unixepoch()))"
]

[[endpoints]]
method = "POST"
path = "/tasks"
action = "create_task"

[[endpoints]]
method = "GET"
path = "/tasks"
action = "list_tasks"

[[endpoints]]
method = "GET"
path = "/tasks/{id}"
action = "get_task"

[[endpoints]]
method = "PUT"
path = "/tasks/{id}"
action = "update_task"

[[endpoints]]
method = "DELETE"
path = "/tasks/{id}"
action = "delete_task"

[actions.create_task]
sql = "INSERT INTO tasks (title) VALUES ($1) RETURNING id, title, done, created_at, updated_at"
params = ["title"]
result = "one"
status = 201

[actions.list_tasks]
sql = "SELECT id, title, done, created_at, updated_at FROM tasks ORDER BY id"
result = "many"

[actions.get_task]
sql = "SELECT id, title, done, created_at, updated_at FROM tasks WHERE id = $1"
params = ["id"]
result = "one"

[actions.update_task]
sql = "UPDATE tasks SET title = $1, done = $2, updated_at = unixepoch() WHERE id = $3 RETURNING id, title, done, created_at, updated_at"
params = ["title", "done", "id"]
result = "one"

[actions.delete_task]
sql = "DELETE FROM tasks WHERE id = $1 RETURNING id"
params = ["id"]
result = "one"
```

## Run and use it

```sh
crudo --config ./tasks.toml
curl -X POST http://127.0.0.1:3000/tasks -H 'content-type: application/json' -d '{"title":"write docs"}'
curl http://127.0.0.1:3000/tasks
curl http://127.0.0.1:3000/tasks/1
curl -X PUT http://127.0.0.1:3000/tasks/1 -H 'content-type: application/json' -d '{"title":"publish docs","done":1}'
curl -X DELETE http://127.0.0.1:3000/tasks/1
```

The create response is `201`; reads and updates are `200`:

```json
{"id":1,"title":"write docs","done":0,"created_at":1784232000,"updated_at":1784232000}
```

`GET`, `PUT`, and `DELETE` for a missing task return `404 {"error":"resource not found"}` because each action uses `result = "one"`.
