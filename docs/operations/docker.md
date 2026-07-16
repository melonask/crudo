# Docker

## Runtime image requirements

The runtime image runs as an unprivileged user, listens on port `3000`, and ships its minimal configuration at `/etc/crudo/config.toml`.

## Starter container

The default SQLite API needs no environment variables.

```sh
docker run --rm -p 3000:3000 ghcr.io/melonask/crudo:latest
curl http://127.0.0.1:3000/v1/health
```

## PostgreSQL wallet demo

Create a network and start PostgreSQL:

```sh
docker network create crudo
docker run -d --name pg --network crudo \
  -e POSTGRES_DB=crudo \
  -e POSTGRES_USER=crudo \
  -e POSTGRES_PASSWORD='replace-me' \
  postgres:18.4-alpine3.24
```

Run the demo with its mounted configuration and required expansions:

```sh
docker run --rm -p 3000:3000 --network crudo \
  -e DATABASE_URL='postgres://crudo:replace-me@pg:5432/crudo' \
  -e WALLET_MNEMONIC \
  -e ALTCHA_SECRET \
  -e ALTCHA_KEY_SECRET \
  -v "$PWD/config/postgres.toml:/etc/crudo/config.toml:ro" \
  ghcr.io/melonask/crudo:latest
```

## Production checklist

- Use a secret manager rather than command history for the mnemonic, database URL, and ALTCHA secrets.
- No `WALLET_PASSPHRASE` is needed unless the mounted configuration references it.
- To avoid wallet variables, mount a configuration without `[wallets]` or wallet action stages.
