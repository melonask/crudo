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
The demo has 24-hour tokens, confirmed deposits and expenses, and public user reads. Full-demo configuration reads `ALTCHA_SECRET` and `ALTCHA_KEY_SECRET`; supply independent high-entropy values through a secret manager.

Documentation contains no production mnemonic recommendation.
:::

## Starter boundary

The built-in starter enables transactional setup and body, timeout, concurrency, and per-IP limits. It does not enable Basic/Bearer authentication, ALTCHA, or CORS.

Add authentication and owner-scoped SQL before exposing equivalent CRUD routes publicly. See [deployment](/operations/deployment) for proxy, TLS, and replica boundaries.
