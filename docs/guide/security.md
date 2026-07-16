# Security model

Use parameter binding, never interpolate request values into SQL. Keep action SQL narrowly authorized by database credentials. Hash passwords via `hash`; configure Basic auth to select a password-hash column and Bearer auth to resolve an owner column.

Enable ALTCHA on abuse-prone anonymous endpoints, use independent high-entropy `secret` and `key_secret`, and restrict CORS origins. Set body, timeout, concurrency, and per-IP rate limits for realistic traffic. See [deployment](/operations/deployment) for proxy, TLS, and replica boundaries.

The demo policy is deliberately illustrative: 24-hour tokens, confirmed deposits and expenses, and public user reads. Full demo configuration obtains ALTCHA secrets from `ALTCHA_SECRET` and `ALTCHA_KEY_SECRET`; supply independent high-entropy values through a secret manager. Replace demo policy choices for production; documentation contains no production mnemonic recommendation.

`config/minimal.toml` is deliberately zero-secret and unauthenticated so a local checkout starts immediately. Its loopback bind, parameter binding, and load limits reduce local risk, but they are not an authorization policy. Add authentication and owner-scoped SQL before exposing equivalent CRUD routes publicly.
