# crudo

Configuration-driven JSON APIs backed by SQL. Routes, authentication, request
protection, schema setup, and result shapes are declared in TOML rather than
application-specific Rust code.

## Install

### crates.io

Install the latest release with Cargo:

```sh
cargo install crudo --locked
```

This requires Rust 1.97 or newer. Upgrade an existing installation by running
the same command again with `--force`.

### Homebrew

The repository includes a source formula for the current `main` branch. Add the
repository as a custom tap, then install its HEAD formula:

```sh
brew tap melonask/crudo https://github.com/melonask/crudo
brew install --HEAD melonask/crudo/crudo
```

Run `brew upgrade --fetch-HEAD melonask/crudo/crudo` to update it. Tagged
releases remain available through crates.io while the project is being prepared
for a versioned Homebrew tap.

### Docker

Run the published multi-platform image without installing crudo:

```sh
docker volume create crudo-data
docker run --rm -p 3000:3000 \
  -v crudo-data:/data \
  ghcr.io/melonask/crudo:latest \
  --config /app/config/sqlite.toml --address 0.0.0.0:3000
```

The default image configuration uses SQLite and creates `/data/crudo.db` on
first launch. Pin a release tag instead of `latest` in production. The image is
published for `linux/amd64` and `linux/arm64`.

## Quick start

Clone the repository to use the documented example configuration, then start
the server:

```sh
crudo --config config/sqlite.toml
curl http://127.0.0.1:3000/v1/users
```

Server startup runs the configured idempotent setup statements in a transaction
before accepting requests. A fresh database is prepared automatically, while an
existing database is left ready to serve by the same safe statements. A failed
statement rolls back the complete setup and prevents the server from starting.

Select PostgreSQL explicitly and supply its URL through the environment:

```sh
DATABASE_URL=postgres://crudo:crudo@localhost:5432/crudo \
  crudo --config config/postgres.toml
```

A configuration may also be loaded over HTTPS:

```sh
crudo --config https://example.com/crudo.toml
```

`${VARIABLE}` references in TOML are expanded before parsing. A missing
variable is a startup error.

## Project structure

- `src/main.rs`: command-line parsing and process entry point.
- `src/app.rs`: TCP serving, shutdown handling, and testable server startup.
- `src/config.rs`: typed configuration, defaults, validation inputs, and loading.
- `src/database.rs`: SQLx connection, setup, value binding, and JSON conversion.
- `src/server.rs`: route construction, actions, authentication, ALTCHA, and limits.
- `config/sqlite.toml`: documented SQLite schema, triggers, routes, and actions.
- `config/postgres.toml`: documented PostgreSQL equivalent using trigger functions.
- `tests/router.rs`: fast in-process integration tests.
- `tests/e2e.rs`: backend-parameterized real-TCP lifecycle test.
- `Dockerfile` and `compose.yaml`: runtime image and reproducible e2e services.

`lib.rs` exposes the small API needed by the binary and integration tests. The
binary owns CLI concerns, while `app.rs` owns process lifecycle concerns.

## Configuration

The checked-in TOML files are complete references and contain comments beside
security-sensitive and dialect-specific values.

### Server and limits

```toml
[server]
address = "127.0.0.1:3000"
prefix = "v1"

[server.limits]
body_bytes = 1048576
timeout_seconds = 30
concurrency = 100
requests = 120
window_seconds = 60
```

The prefix applies to configured routes and the ALTCHA challenge. Limits are
inherited by every endpoint. Override only selected values with an
`[endpoints.limits]` table directly after an endpoint:

```toml
[[endpoints]]
method = "POST"
path = "/imports"
action = "create_import"

[endpoints.limits]
body_bytes = 16384
concurrency = 5
requests = 6
window_seconds = 60
```

Rate counters are independent per endpoint and direct peer IP. `requests = 0`
disables rate limiting. A rejection returns `429 Too Many Requests` and a
`Retry-After` header. Put public-IP limiting at the reverse proxy too when the
direct peer seen by crudo is the proxy.

### Endpoints and actions

An endpoint selects an action. Action parameters are bound in declared order:

```toml
[[endpoints]]
method = "POST"
path = "/users"
action = "create_user"

[actions.create_user]
sql = "INSERT INTO users (name, email, password) VALUES (?, ?, ?) RETURNING id, name, email"
params = ["name", "email", "password"]
hash = ["password"]
result = "one"
status = 201
```

Inputs are merged from path parameters, query parameters, and a JSON object;
JSON values take precedence. `hash` replaces string inputs with Argon2 hashes.
`$owner` is supplied by authentication and `$token` generates a random token.

Result modes are:

- `execute`: return `rows_affected`.
- `one`: require and return one row; no row becomes `404`.
- `optional`: return one row or JSON `null`.
- `many`: return an array.

SQL is backend-native. SQLite uses `?`; PostgreSQL uses `$1`, `$2`, and so on.
Path and query inputs are strings, so PostgreSQL SQL must cast them when used
against numeric columns, for example `$1::BIGINT`.

### Authentication

Basic and Bearer authentication each run configured SQL and resolve an owner:

```toml
[auth.basic]
sql = "SELECT id, password FROM users WHERE email = ?"
owner = "id"
password = "password"

[auth.bearer]
sql = "SELECT user_id FROM sessions WHERE token = ? AND expires_at > unixepoch()"
owner = "user_id"
```

An endpoint declares accepted methods with `auth = ["basic", "bearer"]`.
`auth_optional = true` allows an absent authorization header; a supplied but
invalid header still returns `401 Unauthorized`.

### ALTCHA

```toml
[altcha]
path = "/challenge"
secret = "${ALTCHA_SECRET}"
key_secret = "${ALTCHA_KEY_SECRET}"
algorithm = "PBKDF2/SHA-256"
cost = 5000
max_number = 10000
expires_seconds = 300
bind_ip = true

[[endpoints]]
method = "POST"
path = "/users"
action = "create_user"
altcha = true
```

Fetch a challenge from `GET /v1/challenge`. Submit the solved ALTCHA v2 payload
as Base64 in the JSON `altcha` field. Proofs are signed, expire, are bound to the
direct peer IP by default, and are accepted once per server process. Challenge
and token responses use `Cache-Control: no-store`.

For optional authentication, `altcha_for_authenticated = false` allows valid
authenticated callers to skip ALTCHA while anonymous callers must provide it.
Replay tracking is process-local; multi-replica deployments need sticky routing
or shared replay state if once-per-deployment semantics are required.

## Financial triggers

Both supplied backend configurations implement the same new schema and
behavior. They are separate because trigger and placeholder syntax is
backend-specific.

- Money is stored as integer smallest units in `users.balance`.
- Provider `external_id` values are unique in `transactions` and `expenses`.
- Confirmed deposits increase one user balance and set `credited_at` atomically.
- Confirmed expenses require sufficient balance, debit it, and set `debited_at`.
- Pending rows apply only on their first transition to `confirmed`.
- Later status toggles cannot apply the same row again.
- Missing users, duplicate IDs, and insufficient balances roll back the event.
- Concurrent events use database row/transaction locking rather than process state.

SQLite implements guarded `AFTER` triggers. PostgreSQL implements `BEFORE`
trigger functions that assign processed markers on the row being written. This
project intentionally supports only the current schemas and parameters; it has
no legacy migration or compatibility layer.

## Docker

The checked-in baseline uses the latest stable images available when tested:

- Rust `1.97.0` on Alpine `3.23` for compilation and tests.
- PostgreSQL `18.4` on Alpine `3.23` for database e2e coverage.
- A `scratch` production stage containing only the static application binary
  and configuration. It has no shell, package manager, libc layer, or root user.
- A minimal Alpine e2e stage containing only the compiled test executable; Rust,
  Cargo, source files, dependency caches, and build artifacts remain in disposable
  builder stages.

The e2e images are about 24 MiB each on arm64 and share nearly all of their
layers. Exact sizes vary by architecture. BuildKit keeps builder layers in its
separate build cache to accelerate later builds; they are not part of the
runnable images.

Release builds use full LTO, one codegen unit, size optimization, symbol
stripping, and abort-on-panic. The resulting multi-platform image is about
2.4 MiB compressed on arm64; exact size varies by architecture.

Build the production image:

```sh
docker build --target runtime -t crudo:latest .
```

Run it with SQLite by mounting writable storage and an operator-owned config:

```sh
docker run --rm -p 3000:3000 \
  -v "$PWD/data:/data" \
  -v "$PWD/config/sqlite.toml:/app/config/sqlite.toml:ro" \
  crudo:latest --config /app/config/sqlite.toml
```

Before this command, set `server.address = "0.0.0.0:3000"` in the mounted
operator config. The image works from `/data`, so the supplied relative SQLite
URL creates `/data/crudo.db`. On its first launch the container creates the
database schema automatically before serving. The PostgreSQL configuration
already binds all interfaces and reads `DATABASE_URL`.

The runtime executes as numeric UID/GID `10001:10001`. Ensure bind-mounted data
is writable by that identity. Because the image is `scratch`, use a separate
debug container if filesystem or network inspection is required.

## Testing

Install Rust 1.97 or newer and Docker with Compose. Enable the checked-in Git
hook once per clone:

```sh
git config core.hooksPath .githooks
```

The pre-commit hook runs the same formatting, Clippy, and test suite used by CI.
Run it directly at any time:

```sh
./scripts/check.sh
```

Run the complete real-request scenario against both isolated databases:

```sh
docker compose run --build --rm e2e-sqlite
docker compose run --build --rm e2e-postgres
docker compose down
```

The same `real_http_financial_lifecycle` test runs for both backends. It:

- Starts Axum on a real ephemeral TCP listener and sends Reqwest requests.
- Verifies request body rejection, per-endpoint rate limiting, and `Retry-After`.
- Verifies missing, valid, and replayed ALTCHA proofs.
- Registers three users through HTTP.
- Inserts concurrent confirmed top-ups for all users.
- Confirms pending deposits and expenses, then toggles status repeatedly.
- Rejects duplicate provider IDs and expenses with insufficient balance.
- Reads every final balance through HTTP and verifies trigger isolation.

The e2e test is ignored by normal `cargo test` because it requires an explicit
database environment. Compose supplies the backend and database URL and uses a
health check before starting the PostgreSQL test.

## Releases

Tags matching `v*` publish `linux/amd64` and `linux/arm64` images to
`ghcr.io/melonask/crudo`. Creating a GitHub release publishes the matching
package version to crates.io after tests pass. The release tag must be
`v<version>` and match `package.version` in `Cargo.toml`.

Publishing to crates.io requires the `CARGO_REGISTRY_TOKEN` repository secret.
GHCR publishing uses the workflow-scoped `GITHUB_TOKEN`.
