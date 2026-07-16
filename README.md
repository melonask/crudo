# crudo

Configuration-driven JSON APIs backed by SQL. Routes, authentication, request protection, schema setup, and result shapes are declared in TOML rather than application-specific Rust code.

## Install

### crates.io

```sh
cargo install crudo
```

### Homebrew

```sh
brew install melonask/crudo/crudo
```

### Docker

```sh
export POSTGRES_DB=crudo
export POSTGRES_USER=crudo
export POSTGRES_PASSWORD=password
export DATABASE_URL="postgres://$POSTGRES_USER:$POSTGRES_PASSWORD@pg:5432/$POSTGRES_DB"
export WALLET_MNEMONIC="your dedicated BIP-39 mnemonic"
export WALLET_PASSPHRASE="your optional BIP-39 passphrase"

docker network create crudo

docker run -d \
  --name pg \
  --network crudo \
  -e POSTGRES_DB \
  -e POSTGRES_USER \
  -e POSTGRES_PASSWORD \
  postgres:18.4-alpine3.23

until docker exec pg pg_isready -U "$POSTGRES_USER" -d "$POSTGRES_DB"; do sleep 1; done

docker run --rm -p 3000:3000 \
  --network crudo \
  -e DATABASE_URL \
  -e WALLET_MNEMONIC \
  -e WALLET_PASSPHRASE \
  -v "$PWD/config/postgres.toml:/etc/crudo/config.toml:ro" \
  ghcr.io/melonask/crudo:latest
```

On startup, crudo runs the configured idempotent setup statements before accepting requests.

### Web console

Start the API, then serve the included test console:

```sh
python -m http.server 8000
```

Open [http://localhost:8000](http://localhost:8000) to create users, inspect their assigned blockchain addresses, simulate deposits to those addresses, buy goods, inspect balances and activity, or delete an account.

## Usage

Clone the repository to use the documented example configuration, provide a dedicated wallet secret, then start the server:

```sh
WALLET_MNEMONIC="your dedicated BIP-39 mnemonic" \
WALLET_PASSPHRASE="your optional BIP-39 passphrase" \
  crudo --config config/sqlite.toml
curl http://127.0.0.1:3000/v1/users
```

Before accepting requests, crudo runs the configured idempotent setup statements in a transaction. This automatically prepares a fresh database and safely leaves an existing database ready to serve. If any statement fails, crudo rolls back the entire setup and does not start the server.

Select PostgreSQL explicitly and supply its URL through the environment:

```sh
DATABASE_URL=postgres://crudo:crudo@localhost:5432/crudo \
  crudo --config config/postgres.toml
```

A configuration may also be loaded over HTTPS:

```sh
crudo --config https://example.com/crudo.toml
```

`${VARIABLE}` references in TOML are expanded before parsing. A missing variable prevents the server from starting.

## Configuration

The checked-in TOML files are complete configuration references with comments for security-sensitive and database-specific values.

### Server and limits

```toml
[server]
address = "127.0.0.1:3000"
prefix = "v1"

[server.cors]
origins = ["http://127.0.0.1:8000", "http://localhost:8000"]

[server.limits]
body_bytes = 1048576
timeout_seconds = 30
concurrency = 100
requests = 120
window_seconds = 60
```

The prefix applies to every configured route, including the ALTCHA challenge. All endpoints inherit the server limits. To override selected values, add an `[endpoints.limits]` table directly after an endpoint:

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

Rate counters are tracked independently for each endpoint and direct peer IP. Set `requests = 0` to disable rate limiting. Rejected requests return `429 Too Many Requests` with a `Retry-After` header. If crudo runs behind a reverse proxy, configure public-IP rate limiting at the proxy as well.

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

Inputs are merged from path parameters, query parameters, and a JSON object, with JSON values taking precedence. `hash` replaces string inputs with Argon2 hashes, `$owner` is supplied by authentication, and `$token` generates a random token.

An action can also derive and persist configured wallet profiles from a returned database ID. Wallet actions always execute the primary SQL and every address insertion in one transaction:

```toml
[actions.create_user]
sql = "INSERT INTO users (name, email, password) VALUES (?, ?, ?) RETURNING id, name, email"
params = ["name", "email", "password"]
hash = ["password"]
result = "one"
status = 201

[actions.create_user.wallets]
profiles = ["ethereum-mainnet", "solana-mainnet", "bitcoin-mainnet"]
sql = "INSERT INTO user_addresses (user_id, profile, address_index, address, derivation_path) VALUES (?, ?, 0, ?, ?)"
params = ["$result.id", "$profile.name", "$wallet.address", "$wallet.derivation_path"]

[actions.create_user.wallets.values]
user_id = "$result.id"
address_index = "0"
```

The `values` table maps named derivation-path placeholders to unsigned integer result columns or constants. Wallet-stage parameters may reference `$result.<column>`, `$profile.name`, `$profile.caip2`, `$profile.max_addresses`, `$wallet.address`, and `$wallet.derivation_path`. The parent action must use `result = "one"`. A derivation or persistence failure rolls back the primary action.

Result modes are:

- `execute`: return `rows_affected`.
- `one`: require and return one row; no row becomes `404`.
- `optional`: return one row or JSON `null`.
- `many`: return an array.

SQL uses the native syntax of each database. SQLite uses `?`, while PostgreSQL uses `$1`, `$2`, and so on. Path and query inputs are strings, so PostgreSQL statements must cast them when comparing them with numeric columns, for example `$1::BIGINT`.

Expected database failures can be mapped to safe endpoint responses per action. Messages that are not explicitly mapped remain internal server errors:

```toml
[[actions.create_expense.errors]]
database_message = "expense user not found or insufficient balance"
status = 422
message = "insufficient balance"
```

### Wallet profiles

The mnemonic and optional BIP-39 passphrase define the root seed. Keep both in a secret manager or environment variables; never accept them from an API request or commit production values:

```toml
[wallets]
mnemonic = "${WALLET_MNEMONIC}"
passphrase = "${WALLET_PASSPHRASE}"

[[wallets.profiles]]
name = "ethereum-mainnet"
caip2 = "eip155:1"
curve = "secp256k1"
derivation = "bip32"
path = "m/44'/60'/{user_id}'/0/{address_index}"
address_format = "evm"
max_addresses = 5

[[wallets.profiles]]
name = "solana-mainnet"
caip2 = "solana:4sGjMW1sUnHzSxGspuhpqLDx6wiyjNtZ"
curve = "ed25519"
derivation = "slip10"
path = "m/44'/501'/{user_id}'/{address_index}'"
address_format = "base58-public-key"
max_addresses = 5

[[wallets.profiles]]
name = "bitcoin-mainnet"
caip2 = "bip122:000000000019d6689c085ae165831e93"
curve = "secp256k1"
derivation = "bip32"
path = "m/84'/0'/{user_id}'/0/{address_index}"
address_format = "p2wpkh"
network = "mainnet"
max_addresses = 5
```

Profile names are unique, stable storage identifiers. `caip2` describes the network and is available to wallet-stage parameters without requiring it in every address row. Renaming a profile requires a corresponding data migration.

Supported combinations are:

- `secp256k1` + `bip32` + `evm`, producing EIP-55 addresses.
- `ed25519` + `slip10` + `base58-public-key`, using hardened path children.
- `secp256k1` + `bip32` + `p2wpkh`, with `mainnet`, `testnet`, `signet`, or `regtest`.

Profiles are selected entirely through configuration. Adding a network that uses these primitives requires no Rust changes; a new curve, derivation scheme, or address encoding requires implementing that primitive.

Each named path value must be below `2^31`, the maximum unhardened child-number range used by BIP-32 and SLIP-0010. `max_addresses` must be between `1` and `2^31`. The example profiles use the generated user ID as one path level and a zero-based per-profile address index as another. If either value reaches `2^31`, derivation fails and its transaction is rolled back; in practice, configure a much smaller `max_addresses`, and monitor user IDs long before that boundary.

An authenticated endpoint can select a profile by request input without hard-coding profile names:

```toml
[[endpoints]]
method = "POST"
path = "/addresses"
action = "create_address"
auth = ["bearer"]

[actions.create_address]
sql = "INSERT INTO user_addresses (user_id, profile, address_index, address, derivation_path) SELECT ?, ?, CASE WHEN COUNT(*) < ? THEN COALESCE(MAX(address_index) + 1, 0) ELSE 0 END, ?, '' FROM user_addresses WHERE user_id = ? AND profile = ? RETURNING user_id, profile, address_index"
params = ["$owner", "$profile.name", "$profile.max_addresses", "$token", "$owner", "$profile.name"]
result = "one"
status = 201

[actions.create_address.wallets]
profile = "profile"
sql = "UPDATE user_addresses SET address = ?, derivation_path = ? WHERE user_id = ? AND profile = ? AND address_index = ?"
params = ["$wallet.address", "$wallet.derivation_path", "$result.user_id", "$profile.name", "$result.address_index"]

[actions.create_address.wallets.values]
user_id = "$result.user_id"
address_index = "$result.address_index"
```

Here, `profile = "profile"` reads the profile name from the JSON input and rejects names absent from `[wallets.profiles]`. The action SQL atomically allocates the next per-user index and receives the configured limit through `$profile.max_addresses`. At the limit it deliberately selects the existing index zero, causing the table's primary-key constraint to return `409 Conflict`. PostgreSQL uses the atomic counter upsert shown in `config/postgres.toml` to serialize concurrent allocations for one user and profile.

A normalized address table keeps the schema independent of the profile count:

```sql
CREATE TABLE user_addresses (
  user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  profile TEXT NOT NULL,
  address_index BIGINT NOT NULL,
  address TEXT NOT NULL,
  derivation_path TEXT NOT NULL,
  PRIMARY KEY (user_id, profile, address_index),
  UNIQUE (profile, address)
);
```

Only public addresses and derivation metadata are stored. Private keys are deterministically recoverable from the protected mnemonic, passphrase, profile path, and user ID and are never written to the database.

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

An endpoint declares its accepted authentication methods with `auth = ["basic", "bearer"]`. Setting `auth_optional = true` allows requests without an authorization header, but an invalid supplied header still returns `401 Unauthorized`.

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

Fetch a challenge from `GET /v1/challenge`, then submit the solved ALTCHA v2 payload as Base64 in the JSON `altcha` field. Proofs are signed, expire, are bound to the direct peer IP by default, and can be used once per server process. Challenge and token responses use `Cache-Control: no-store`.

For optional authentication, setting `altcha_for_authenticated = false` allows authenticated callers to skip ALTCHA while anonymous callers must provide it. Replay tracking is process-local, so multi-replica deployments require sticky routing or shared replay state for once-per-deployment semantics.
