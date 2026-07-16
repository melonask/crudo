---
layout: home
hero:
  name: crudo
  text: Turn deliberate SQL into a JSON API
  tagline: Define routes, parameter binding, response shapes, setup, and protections in TOML—without application-specific Rust.
  image: { src: '/logo.svg', alt: 'crudo logo' }
  actions:
    - theme: brand
      text: Get started
      link: /guide/getting-started
    - theme: alt
      text: View repository
      link: https://github.com/melonask/crudo
features:
  - title: Config-driven routes
    details: Map an HTTP method and path to a named SQL action; every route is explicit.
  - title: Native SQL binding
    details: Bind request values in declared order with SQLite ? or PostgreSQL $1-style placeholders.
  - title: Predictable JSON
    details: Select execute, one, optional, or many for a stable response shape per action.
  - title: Security and load controls
    details: Configure auth, ALTCHA, CORS, body limits, timeouts, concurrency, and per-IP limits.
  - title: SQLite or PostgreSQL
    details: Start with a local SQLite file or use PostgreSQL with its native SQL syntax.
  - title: Optional deterministic wallets
    details: Wallet stages can derive and persist supported public addresses atomically when configured.
---

## Start a local API with no environment variables

Install and start crudo:

```sh
cargo install crudo
crudo
# Optional: in another terminal
curl http://127.0.0.1:3000/v1/health
```

The starter:

- Writes `crudo.db` in the current writable directory.
- Listens only on loopback for the native CLI.

In another terminal, exercise the complete item lifecycle:

```sh
curl -X POST http://127.0.0.1:3000/v1/items \
  -H 'content-type: application/json' -d '{"name":"tea"}'
curl http://127.0.0.1:3000/v1/items
curl -X PUT http://127.0.0.1:3000/v1/items/1 \
  -H 'content-type: application/json' -d '{"name":"green tea"}'
curl -X DELETE http://127.0.0.1:3000/v1/items/1
```

```jsonc
// health → 200
{"status":"ok"}
// create → 201
{"id":1,"name":"tea","created_at":1784232000,"updated_at":1784232000}
// list → 200
[{"id":1,"name":"tea","created_at":1784232000,"updated_at":1784232000}]
// update → 200
{"id":1,"name":"green tea","created_at":1784232000,"updated_at":1784232010}
// delete → 200
{"id":1}
```

## From TOML to JSON

An endpoint names an action; the action binds `params` in order and determines the JSON response with `result`. This compact SQLite example creates a note from a JSON body.

```toml
[database]
url = "sqlite://notes.db?mode=rwc"
setup = ["CREATE TABLE IF NOT EXISTS notes (id INTEGER PRIMARY KEY AUTOINCREMENT, body TEXT NOT NULL)"]

[[endpoints]]
method = "POST"
path = "/notes"
action = "create_note"

[actions.create_note]
sql = "INSERT INTO notes (body) VALUES (?) RETURNING id, body"
params = ["body"]
result = "one"
status = 201
```

```sh
curl -X POST http://127.0.0.1:3000/notes \
  -H 'content-type: application/json' \
  -d '{"body":"Write the query first"}'
```

```json
{"id":1,"body":"Write the query first"}
```

| `result` mode | Successful JSON response |
|---|---|
| `execute` | `{"rows_affected": n}` |
| `one` | Exactly one row; absent rows return `404` |
| `optional` | One row or `null` |
| `many` | An array of rows |

## Protections are configured, not assumed

| Control | Scope |
|---|---|
| Body size, timeout, concurrency, and per-direct-IP rate limits | Server defaults; endpoints can override them. |
| Basic/Bearer authentication, ALTCHA, and CORS origins | Explicitly configured per API or endpoint. |
| Database setup | Runs transactionally before the server listens. |

Invalid routes, action references, protection settings, and static configuration fields fail startup validation.

## Optional wallets and shipped configurations

::: info Wallet requirements are conditional
Wallets are optional: `WALLET_MNEMONIC` is not required by crudo or by configurations that omit `[wallets]` and wallet action stages.

The full demos require `WALLET_MNEMONIC`, `ALTCHA_SECRET`, and `ALTCHA_KEY_SECRET`. A wallet passphrase defaults to an empty string, so no passphrase variable is needed unless your configuration explicitly uses one.
:::

| Configuration | Database and scope | Required environment |
|---|---|---|
| Built-in starter | Local SQLite health and item CRUD starter | None |
| `config/sqlite.toml` | Full SQLite demo with ALTCHA and wallet stages | `WALLET_MNEMONIC`, `ALTCHA_SECRET`, `ALTCHA_KEY_SECRET` |
| `config/postgres.toml` | Full PostgreSQL demo with ALTCHA and wallet stages | `DATABASE_URL` plus the three values above |

## Choose your next step

### Start

- **[Getting started](/guide/getting-started)** — run and adapt the local starter.
- **[curl lifecycle](/examples/curl)** — create, list, read, update, and delete an item.

### Design

- **[Custom CRUD API](/examples/custom-crud)** — copy a small SQLite tasks service.
- **[Configuration schema](/reference/configuration)** and **[actions](/reference/actions)** — choose fields, binding, and response modes.

### Protect & deploy

- **[Authentication example](/examples/authentication)** — issue a Bearer token from Basic credentials and scope SQL by owner.
- **[Limits and errors](/examples/limits-errors)** — set resource limits and map expected database failures.
- **[Security](/guide/security)** and **[deployment](/operations/deployment)** — set production boundaries.

### Explore

- **[SQLite](/examples/sqlite)** and **[PostgreSQL](/examples/postgresql)** — compare shipped demo configurations.
- **[Demo API](/reference/demo-api)**, **[Docker](/operations/docker)**, and **[GitHub](https://github.com/melonask/crudo)** — inspect the full demo, packaging, and source.

## Common recipes

**Use a local config** — `crudo --config ./tasks.toml`

**Load a reviewed HTTPS config** — `crudo --config https://config.example.com/tasks.toml`

**Change only the bind address** — `crudo --address 127.0.0.1:4000`

**Tighten one route and require Bearer auth** — add this to that endpoint:

```toml
auth = ["bearer"]

[endpoints.limits]
requests = 10
window_seconds = 60
```

See [custom CRUD](/examples/custom-crud), [authentication](/examples/authentication), and [limits & errors](/examples/limits-errors) for complete runnable configurations.
