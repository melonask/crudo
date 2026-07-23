# Security model

## SQL and authentication

- Use parameter binding; never interpolate request values into SQL.
- Keep action SQL narrowly authorized by database credentials.
- Hash passwords with `hash`.
- Configure Basic auth to select a password-hash column and Bearer auth to resolve an owner column.

## Request protections

| Control | Use |
|---|---|
| ALTCHA | Protect abuse-prone anonymous endpoints. |
| CORS origins | Restrict browser origins explicitly. |
| Body, timeout, concurrency limits | Bound request resource use. |
| Per-IP limits | Apply realistic direct-IP limits. |

Use independent high-entropy ALTCHA `secret` and `key_secret` values.

## Demo policy

::: warning Replace demo policy in production
The shipped store bootstrap seeds `admin` / `admin` and permits self-service demo credit without payment verification. Change or remove the account, remove the top-up flow, and implement payment verification before exposure. Wallet and ALTCHA secrets are required only when those optional features are configured; supply independent high-entropy ALTCHA values through a secret manager.
:::

## Configuration boundary

Crudo installs no routes or protections by default. Configure transactional setup, limits, authentication, ALTCHA, and CORS deliberately for each API.

Add authentication and owner-scoped SQL before exposing equivalent CRUD routes publicly. See [deployment](/operations/deployment) for proxy, TLS, and replica boundaries.
