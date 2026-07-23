# Shipped store API

`config/sqlite.toml` and `config/postgres.toml` are digital-store bootstraps. Both require `WALLET_MNEMONIC`, `ALTCHA_SECRET`, and `ALTCHA_KEY_SECRET`; PostgreSQL also requires `DATABASE_URL`. Both explicitly set `prefix = "v1"`; every route here is served under `/v1`. They seed four active products and demo-only `admin` / `admin` idempotently without overwriting edits. Change or remove that account before deployment.

::: warning Development only
Each registration derives and persists Base and Solana user wallets. Self-service top-ups create demo credit without payment-provider verification. Insufficient purchases return a configured x402 `402` requirement; Crudo does not verify or settle its payments. Product and transaction setup is a demo bootstrap, not migration tooling or real payment processing. Do not expose it as-is.
:::

## Routes

| Served route | Access | Purpose |
|---|---|---|
| `GET /v1/products` | Public | Active products only; fulfillment and license data are excluded. |
| `GET /v1/challenge` | Public | Issue a short-lived ALTCHA challenge. |
| `GET /v1/payment-methods` | Public | Active exact payment methods, ordered deterministically; frontend source for supported coins and blockchains. |
| `POST /v1/users` | Public + ALTCHA | Register a customer. |
| `POST /v1/tokens` | Basic + ALTCHA | Create a Bearer session token. |
| `GET /v1/me` | Bearer | Current account and balance. |
| `GET /v1/transactions` | Bearer | Caller’s transactions, including that caller’s fulfillment and license key. |
| `POST /v1/top-ups` | Bearer | Add caller demo credit. |
| `POST /v1/purchases` | Bearer | Buy one active product with caller balance. |
| `GET /v1/admin/summary` | Admin Bearer | Store totals. |
| `GET /v1/admin/users` | Admin Bearer | All users. |
| `GET /v1/admin/transactions` | Admin Bearer | All transactions. |
| `GET /v1/admin/users/{id}/transactions` | Admin Bearer | One user’s transactions. |
| `GET /v1/admin/products` | Admin Bearer | All products, including inactive and fulfillment fields. |
| `POST /v1/admin/products` | Admin Bearer | Create a product. |
| `PUT /v1/admin/products/{id}` | Admin Bearer | Replace product fields. |
| `PUT /v1/admin/products/{id}/status` | Admin Bearer | Set a product’s active status. |

There are no public user listings, addresses, wallet routes, expenses, or generic transaction creation routes in these configurations.

## Account and customer flows

Fetch a fresh challenge, solve it, Base64-encode the ALTCHA payload, and include it as `altcha` for every registration and login request. Register with `name`, `email`, `password`, and `altcha`:

```json
{"name":"Ada","email":"ada@example.test","password":"correct horse","altcha":"<Base64 ALTCHA payload>"}
```

`POST /v1/tokens` uses `Authorization: Basic <base64(email:password)>`, a JSON body containing `{"altcha":"<Base64 ALTCHA payload>"}`, and returns a token and expiry. Use it as `Authorization: Bearer <token>` for `/v1/me` and customer routes. Proofs are one-time and IP-bound. Registration returns `201`; token creation returns `201`.

Amounts and prices are integer **cents**. A top-up needs a caller-supplied, nonempty `external_id` and positive `amount`:

```json
{"external_id":"credit-001","amount":5000}
```

A purchase needs its own `external_id` and an active `product_id`:

```json
{"external_id":"order-001","product_id":1}
```

`external_id` is unique across transactions, so it is the idempotency key: do not generate a new value when retrying the same intended operation. A confirmed top-up credits once; a confirmed purchase debits once and rejects insufficient balance with `402`. Its configured x402 v2 payload accepts GLOBAL exact Base USDC and includes user-specific deposit destinations as informational extension data. It is a payment requirement only: Crudo does not verify or settle x402 payments, and `/v1/top-ups` remains demo credit. Purchases snapshot product name, fulfillment, and a generated license key into the buyer’s transaction. Those fulfillment and license fields are not public product data and are visible only to the buyer or an administrator through transaction routes.

`GET /v1/transactions` is owner-scoped. `GET /v1/products` returns only active products and deliberately omits fulfillment and license information.

## Administration

Administrator routes require a Bearer token belonging to a user with `role = "admin"`. Product creation and replacement require `slug`, `name`, `description`, `category` (`license`, `service`, `book`, or `asset`), positive integer `price` in cents, and `fulfillment`. Status updates send:

```json
{"active":false}
```

The SQLite configuration enforces its administrator predicate in SQL: a non-admin receives safe empty arrays or `null` for optional mutation results. PostgreSQL raises and maps the same condition to `403`. Clients must treat either behavior as denied access and must not infer authorization from an empty result.

## Store frontend

The frontend lives at [demo-crudo.github.io](https://demo-crudo.github.io/) and its source is [demo-crudo/demo-crudo.github.io](https://github.com/demo-crudo/demo-crudo.github.io), not this repository. Its visible **API URL** field accepts any compatible API base and defaults exactly to `http://127.0.0.1:3000/v1`. It is keyboard-accessible and provides a minimal customer dashboard (balance, demo credit, purchases, own history) or administrator dashboard (summary, users, transactions, products).

The store configurations permit the hosted UI at `https://demo-crudo.github.io` plus local development at `http://127.0.0.1:8000` and `http://localhost:8000`. Custom deployments must configure their own exact origins; use existing `${ENV}` expansion in TOML when appropriate rather than changing Rust. Independently of CORS, an HTTPS page targeting plain HTTP localhost may be blocked by browser local-network or mixed-content policy. Crudo does not serve or hardcode the frontend.
