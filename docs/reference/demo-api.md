# Shipped full-demo API

`config/sqlite.toml` and `config/postgres.toml` mount these routes at `/v1`. They are full demos, not a production policy: reads of users are public; registration requires ALTCHA; tokens last 86,400 seconds; deposits and expenses are inserted as `confirmed`. These files require `WALLET_MNEMONIC`, `ALTCHA_SECRET`, and `ALTCHA_KEY_SECRET`; no wallet passphrase variable is needed unless you add one to configuration.

| Route | Protection | Success |
|---|---|---|
| `GET /challenge` | none | ALTCHA challenge, 200/no-store |
| `POST /users` | ALTCHA | user, 201; initial wallet rows persist transactionally |
| `GET /users` | public | users, 200 |
| `GET /users/{id}` | public | user, 200; 404 if absent |
| `POST /tokens` | Basic | token, 201/no-store |
| `DELETE /users/{id}` | Basic or Bearer | user or `null`, 200 |
| `GET /transactions` | Bearer | caller transactions, 200 |
| `GET /addresses` | Bearer | caller addresses, 200 |
| `POST /addresses` | Bearer | new selected-profile address, 201 |
| `POST /transactions` | Bearer | confirmed deposit, 201 |
| `GET /expenses` | Bearer | caller expenses, 200 |
| `POST /expenses` | Bearer | confirmed expense, 201; insufficient funds 422 |

## Representative exchanges

`GET /challenge` returns an ALTCHA v2 challenge object such as `{"algorithm":"PBKDF2/SHA-256","challenge":"…","salt":"…","signature":"…","maxnumber":10000,"expires":1760000300}` with `Cache-Control: no-store`. Field values, including challenge, salt, signature, and expiry, are illustrative and change per request. Solve it with an ALTCHA-compatible client and encode the payload as Base64.

Registration body is `{"name":"Ada","email":"ada@example.test","password":"correct horse","altcha":"<base64 proof>"}`. Response is only `{"id":1,"name":"Ada","email":"ada@example.test"}` (201). The wallet stage persists the initial profile addresses in the same transaction but does not add them to this response. Omitted/reused proof returns `403 {"error":"invalid, expired, or reused ALTCHA proof"}`; a duplicate email returns 409.

Public user list returns `[{"id":1,"name":"Ada","email":"ada@example.test","balance":375}]`; `GET /users/1` returns that object. A missing ID returns `404 {"error":"resource not found"}`. `DELETE /users/1`, authorized as that user, returns `{"id":1,"name":"Ada","email":"ada@example.test"}`; deleting an absent/non-owned user returns `null` with 200 because this action uses `optional`.

Create a token with `Authorization: Basic <base64(email:password)>`; the 201/no-store response is `{"token":"illustrative-issued-token"}`. Use that returned value as `Authorization: Bearer …`; it is not a stable or predictable token. Invalid credentials return 401.

Authenticated addresses list as `[{"profile":"ethereum-mainnet","address_index":0,"address":"0xillustrativeAddress","derivation_path":"m/44'/60'/1'/0/0"}]`. Creation body is `{"profile":"ethereum-mainnet"}`; its 201 response is `{"user_id":1,"profile":"ethereum-mainnet","address_index":1}`. Actual addresses are derived values, not the illustrative value shown here. Unknown profiles return 400; profile index conflicts return 409.

Transactions list returns `[{"id":7,"external_id":"chain-tx-42","profile":"ethereum-mainnet","address":"0xillustrativeAddress","type":"deposit","status":"confirmed","amount":500,"credited_at":1760000000,"created_at":1760000000}]`. Deposit body is `{"external_id":"chain-tx-42","profile":"ethereum-mainnet","address":"0xillustrativeAddress","amount":500}`; its 201 response has the same fields for the new row. The profile/address must match one of the caller's stored addresses.

Expenses list returns `[{"id":3,"external_id":"order-42","status":"confirmed","amount":125,"description":"Subscription","debited_at":1760000010,"created_at":1760000010}]`. Expense body is `{"external_id":"order-42","amount":125,"description":"Subscription"}` and returns that object with 201. Reusing either external ID yields 409; too little balance yields `422 {"error":"insufficient balance"}`. Deposit triggers credit once; expense triggers debit once, even if later statuses change.
