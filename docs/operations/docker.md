# Docker

The published runtime image is built from Rust 1.97.0 on Alpine 3.24, runs as an unprivileged user, listens on port 3000, and ships the minimal configuration as its default. It needs no environment variables for that default SQLite API:

The minimal API is an unauthenticated starter. Keep it on a development host or replace it with a configuration that applies your authentication and authorization policy before exposing it publicly.

```sh
docker run --rm -p 3000:3000 ghcr.io/melonask/crudo:latest
curl http://127.0.0.1:3000/v1/health
```

To run the PostgreSQL wallet demo instead, mount its configuration and provide its required expansions:

```sh
docker network create crudo
docker run -d --name pg --network crudo -e POSTGRES_DB=crudo -e POSTGRES_USER=crudo -e POSTGRES_PASSWORD='replace-me' postgres:18.4-alpine3.24
docker run --rm -p 3000:3000 --network crudo \
  -e DATABASE_URL='postgres://crudo:replace-me@pg:5432/crudo' \
  -e WALLET_MNEMONIC -e ALTCHA_SECRET -e ALTCHA_KEY_SECRET \
  -v "$PWD/config/postgres.toml:/etc/crudo/config.toml:ro" \
  ghcr.io/melonask/crudo:latest
```

Use a secret manager rather than command history for mnemonic, database URL, and ALTCHA secrets. No `WALLET_PASSPHRASE` is needed unless your mounted configuration references it. To avoid wallet variables, mount a configuration without `[wallets]` or wallet action stages.
