# Deployment

## Requirements

- Use least-privileged database credentials.
- Inject production secrets securely.
- Set resource limits, backups, and migrations appropriate to the database engine.
- Treat startup database setup as transactional setup, not as a migration framework.
- Select a reviewed production configuration explicitly with `--config`; do not rely on the working directory's `Crudo.toml`.
- Set an explicit production database URL. Omitting `[database]` uses local `sqlite://crudo.db?mode=rwc` and runs no setup statements.
- Do not deploy the shipped store bootstrap unchanged: change or remove its seeded `admin` / `admin`, remove self-service demo top-ups, and replace bootstrap setup with managed migrations and payment-provider flows.
- `config/store.toml` requires `DATABASE_URL`, `WALLET_MNEMONIC`, `ALTCHA_SECRET`, and `ALTCHA_KEY_SECRET`; generate, inject, and rotate secrets securely.

## Proxy and TLS checklist

- Terminate TLS at a trusted reverse proxy or load balancer.
- Forward only validated traffic to crudo.
- Keep the default loopback address (`127.0.0.1:3000`) unless the deployment's network controls require another bind address; restrict firewalls accordingly.
- Configure exact CORS origins.
- Ensure health checks do not consume protected endpoints.
- The optional store frontend is published at [demo-crudo.github.io](https://demo-crudo.github.io/) and sourced from [demo-crudo/demo-crudo.github.io](https://github.com/demo-crudo/demo-crudo.github.io); Crudo serves only configured API routes. Its visible **API URL** field accepts any compatible API base and defaults exactly to `http://127.0.0.1:3000/v1`.
- The shipped store configuration explicitly sets `prefix = "v1"` and allows `https://demo-crudo.github.io` plus `http://127.0.0.1:8000` and `http://localhost:8000`. Custom deployments must configure their own exact origins; TOML supports existing `${ENV}` expansion, so origins can be deployment configuration without changing Rust. Independently of CORS, an HTTPS page targeting plain HTTP localhost may be blocked by browser local-network or mixed-content policy.

::: warning Direct-peer IP behavior
Crudo's rate limiter and ALTCHA IP binding use the direct TCP peer. Behind a proxy, that is normally the proxy address rather than the browser.

Apply public-client IP rate limiting at the proxy.
:::

## Replica checklist

- Use sticky routing if IP-bound ALTCHA is exposed through multiple replicas.
- Enforce shared or public-IP policy at the proxy.
- Do not assume global rate limits or global proof replay protection.

Rate counters, ALTCHA replay records, and challenge single-use state are process-local.

## Anti-automation scope

ALTCHA is application-layer anti-automation, not volumetric DDoS protection. The shipped limits bound server concurrency, request bodies and timeouts, challenge issuance and public reads, and registration/login attempts; retain authenticated mutation limits. Keep rate limits in place and use reverse-proxy and network-level rate limiting, connection controls, and DDoS protections for public exposure.

## Database topology

| Deployment | Guidance |
|---|---|
| Local or single host | SQLite is appropriate. |
| Concurrent or multi-replica | Use careful shared-storage locking or PostgreSQL. |

Use TLS for every relevant trust boundary.
