# Deployment

## Requirements

- Use least-privileged database credentials.
- Inject production secrets securely.
- Set resource limits, backups, and migrations appropriate to the database engine.
- Treat startup database setup as transactional setup, not as a migration framework.

## Proxy and TLS checklist

- Terminate TLS at a trusted reverse proxy or load balancer.
- Forward only validated traffic to crudo.
- Restrict bind addresses and firewalls.
- Configure exact CORS origins.
- Ensure health checks do not consume protected endpoints.

::: warning Direct-peer IP behavior
Crudo's rate limiter and ALTCHA IP binding use the direct TCP peer. Behind a proxy, that is normally the proxy address rather than the browser.

Apply public-client IP rate limiting at the proxy.
:::

## Replica checklist

- Use sticky routing if IP-bound ALTCHA is exposed through multiple replicas.
- Enforce shared or public-IP policy at the proxy.
- Do not assume global rate limits or global proof replay protection.

Rate counters, ALTCHA replay records, and challenge single-use state are process-local.

## Database topology

| Deployment | Guidance |
|---|---|
| Local or single host | SQLite is appropriate. |
| Concurrent or multi-replica | Use careful shared-storage locking or PostgreSQL. |

Use TLS for every relevant trust boundary.
