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
    details: Bind request values in declared order with universal `$1`, `$2`, and higher placeholders.
  - title: Predictable JSON
    details: Select execute, one, optional, or many for a stable response shape per action.
  - title: Security and load controls
    details: Configure auth, ALTCHA, CORS, body limits, timeouts, concurrency, and per-IP limits.
  - title: SQLite or PostgreSQL
    details: Start with a local SQLite file or use PostgreSQL with its native SQL syntax.
  - title: Optional deterministic wallets
    details: Wallet stages can derive and persist supported public addresses atomically when configured.
---

## Start a local API

Install crudo and create `./Crudo.toml`:

```sh
cargo install crudo
```

```toml
[[endpoints]]
method = "GET"
path = "/health"
action = "health"

[actions.health]
sql = "SELECT 'ok' AS status"
result = "one"
```

```sh
crudo
curl http://127.0.0.1:3000/health
```

Without `[database]`, crudo uses local `sqlite://crudo.db?mode=rwc`. Without `[server]`, it listens on `127.0.0.1:3000` with an empty prefix, so this endpoint is exactly `/health`. `--config` explicitly selects a local path or HTTPS URL; if it and `./Crudo.toml` are absent, startup fails with guidance.

## From TOML to JSON

An endpoint names an action; the action binds `params` in order and determines the JSON response with `result`. This compact SQLite example creates a note from a JSON body.

```toml
[database]
url = "sqlite://notes.db?mode=rwc"

[database.setup.sqlite]
statements = ["CREATE TABLE IF NOT EXISTS notes (id INTEGER PRIMARY KEY AUTOINCREMENT, body TEXT NOT NULL)"]

[[endpoints]]
method = "POST"
path = "/notes"
action = "create_note"

[actions.create_note]
sql = "INSERT INTO notes (body) VALUES ($1) RETURNING id, body"
params = ["body"]
result = "one"
status = 201
```

This example omits `server.prefix`, so its route is `/notes`. Set `prefix = "v1"` under `[server]` only when it should be `/v1/notes`.

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

ALTCHA is independent of the shipped store configuration. It uses wallet stages and requires `WALLET_MNEMONIC`; a wallet passphrase defaults to an empty string when wallets are configured.
:::

| Configuration | Database and scope | Required environment |
|---|---|---|
| `config/minimal.toml` | Explicit local SQLite health and item CRUD example; `prefix = "v1"` | None |
| `config/store.toml` | Universal digital-store bootstrap; select SQLite or PostgreSQL with `DATABASE_URL`; `prefix = "v1"` | `DATABASE_URL`, `WALLET_MNEMONIC`, `ALTCHA_SECRET`, `ALTCHA_KEY_SECRET` |

## Choose your next step

### Start

- **[Getting started](/guide/getting-started)** — create and run a local configuration.
- **[curl lifecycle](/examples/curl)** — run the explicit minimal CRUD example.

### Design

- **[Custom CRUD API](/examples/custom-crud)** — copy a small SQLite tasks service.
- **[Configuration schema](/reference/configuration)** and **[actions](/reference/actions)** — choose fields, binding, and response modes.

### Protect & deploy

- **[Authentication example](/examples/authentication)** — issue a Bearer token from Basic credentials and scope SQL by owner.
- **[Limits and errors](/examples/limits-errors)** — set resource limits and map expected database failures.
- **[Security](/guide/security)** and **[deployment](/operations/deployment)** — set production boundaries.

### Explore

- **[Store](/examples/store)** — run the universal shipped demo with SQLite or PostgreSQL.
- **[Live store demo](https://demo-crudo.github.io/)**, **[Store API](/reference/demo-api)**, **[Docker](/operations/docker)**, and **[GitHub](https://github.com/melonask/crudo)** — inspect the separately hosted frontend, store bootstrap, packaging, and source.

## Common recipes

**Use `./Crudo.toml` automatically** — `crudo`

**Use a local config explicitly** — `crudo --config ./tasks.toml`

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
