# Deployment

Terminate TLS at a trusted reverse proxy or load balancer and forward only validated traffic to crudo. Crudo's rate limiter and ALTCHA IP binding use the direct TCP peer; behind a proxy that is normally the proxy address, not the browser. Apply client-IP rate limiting at the proxy and use sticky routing if IP-bound ALTCHA is exposed through multiple replicas.

Rate counters, ALTCHA replay records, and challenge single-use state are process-local. Multi-replica deployment does not provide global limits or once-per-deployment proof replay protection without shared state/proxy enforcement. Use TLS, restrict bind addresses/firewalls, set exact CORS origins, and ensure health checking does not consume protected endpoints.

Use least-privileged database credentials, backups and migrations appropriate to your engine, production secret injection, and resource limits. Startup database setup is transactional but is not a migration framework. SQLite is appropriate for local/single-host use; concurrent and multi-replica deployments need careful shared-storage locking or PostgreSQL.
