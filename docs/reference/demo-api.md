# Shipped full-demo API

`config/sqlite.toml` and `config/postgres.toml` mount these routes at `/v1`.

::: warning Demo policy
These files demonstrate crudo's capabilities. Their public reads, token lifetime, and financial behavior are not a production-ready authorization or accounting policy.
:::

::: info Required full-demo environment
Both full demos require `WALLET_MNEMONIC`, `ALTCHA_SECRET`, and `ALTCHA_KEY_SECRET`. No wallet passphrase variable is needed unless you add one to configuration.
:::

## Endpoints

| Route | Protection | Success |
|---|---|---|
| `GET /challenge` | None | ALTCHA challenge, `200`, no-store |
| `POST /users` | ALTCHA | User, `201`. Initial wallet rows persist transactionally. |
| `GET /users` | Public | Users, `200` |
| `GET /users/{id}` | Public | User, `200`; `404` if absent. |
| `POST /tokens` | Basic | Token, `201`, no-store |
| `DELETE /users/{id}` | Basic or Bearer | User or `null`, `200` |
| `GET /transactions` | Bearer | Caller transactions, `200` |
| `GET /addresses` | Bearer | Caller addresses, `200` |
| `POST /addresses` | Bearer | New selected-profile address, `201` |
| `POST /transactions` | Bearer | Confirmed deposit, `201` |
| `GET /expenses` | Bearer | Caller expenses, `200` |
| `POST /expenses` | Bearer | Confirmed expense, `201`; insufficient funds `422`. |

## Registration and challenge

`GET /challenge` returns an ALTCHA v2 challenge. The values change per request.

```json
{"algorithm":"PBKDF2/SHA-256","challenge":"…","salt":"…","signature":"…","maxnumber":10000,"expires":1760000300}
```

Solve the challenge with an ALTCHA-compatible client and Base64-encode its payload.

Send that value in the registration body:

```json
{"name":"Ada","email":"ada@example.test","password":"correct horse","altcha":"<base64 proof>"}
```

The `201` response contains the user, not the persisted initial wallet rows.

```json
{"id":1,"name":"Ada","email":"ada@example.test"}
```

Omitted or reused proof returns `403 {"error":"invalid, expired, or reused ALTCHA proof"}`. A duplicate email returns `409`.

## Auth

Create a token with `Authorization: Basic <base64(email:password)>`.

```json
{"token":"illustrative-issued-token"}
```

Response metadata:

- Status: `201`
- Header: `Cache-Control: no-store`

Use the returned value as `Authorization: Bearer …`. The token is neither stable nor predictable. Invalid credentials return `401`.

## Users

Public user reads are part of the demo policy.

```json
[{"id":1,"name":"Ada","email":"ada@example.test","balance":375}]
```

`GET /users/1` returns the corresponding object. A missing ID returns:

```json
{"error":"resource not found"}
```

Deleting user `1` as that user returns:

```json
{"id":1,"name":"Ada","email":"ada@example.test"}
```

Deleting an absent or non-owned user returns `null` with `200`, because the action uses `optional`.

## Addresses

List the caller's addresses with `GET /addresses`.

```json
[{"profile":"ethereum-mainnet","address_index":0,"address":"0xillustrativeAddress","derivation_path":"m/44'/60'/1'/0/0"}]
```

Create an address by selecting a profile:

```json
{"profile":"ethereum-mainnet"}
```

The `201` response identifies the allocated row.

```json
{"user_id":1,"profile":"ethereum-mainnet","address_index":1}
```

Actual addresses are derived values, not the illustrative value above. Unknown profiles return `400`; profile-index conflicts return `409`.

## Transactions

`GET /transactions` returns the caller's transactions.

```json
[{"id":7,"external_id":"chain-tx-42","profile":"ethereum-mainnet","address":"0xillustrativeAddress","type":"deposit","status":"confirmed","amount":500,"credited_at":1760000000,"created_at":1760000000}]
```

Create a confirmed deposit with this body:

```json
{"external_id":"chain-tx-42","profile":"ethereum-mainnet","address":"0xillustrativeAddress","amount":500}
```

The `201` response has the same fields for the new row. The profile and address must match one of the caller's stored addresses. A deposit credits once, even if a later status changes.

## Expenses

`GET /expenses` returns the caller's expenses.

```json
[{"id":3,"external_id":"order-42","status":"confirmed","amount":125,"description":"Subscription","debited_at":1760000010,"created_at":1760000010}]
```

Create a confirmed expense with this body:

```json
{"external_id":"order-42","amount":125,"description":"Subscription"}
```

The `201` response returns that object. Reusing an external ID returns `409`; insufficient balance returns:

```json
{"error":"insufficient balance"}
```

An expense debits once, even if a later status changes.
