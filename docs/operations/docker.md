# Docker

## Runtime image requirements

The runtime image runs as an unprivileged user, listens on port `3000`, and ships its minimal configuration at `/etc/crudo/config.toml`.

## Minimal API container

The image explicitly runs the packaged `config/minimal.toml`, which configures `prefix = "v1"` and needs no environment variables.

```sh
docker run --rm -p 3000:3000 ghcr.io/melonask/crudo:latest
curl http://127.0.0.1:3000/v1/health
```

## PostgreSQL store demo

Create a network and start PostgreSQL:

```sh
docker network create crudo
docker run -d --name pg --network crudo \
  -e POSTGRES_DB=crudo \
  -e POSTGRES_USER=crudo \
  -e POSTGRES_PASSWORD='replace-me' \
  postgres:18.4-alpine3.24
```

The image defaults to `minimal.toml`; it does not package the store configurations as its default. Mount the store configuration explicitly. `config/postgres.toml` requires `DATABASE_URL` and `WALLET_MNEMONIC`, and explicitly configures `prefix = "v1"`.

```sh
docker run --rm -p 3000:3000 --network crudo \
  -e DATABASE_URL \
  -e WALLET_MNEMONIC \
  -v "$PWD/config/postgres.toml:/etc/crudo/config.toml:ro" \
  ghcr.io/melonask/crudo:latest
```

## Production checklist

- Provide `DATABASE_URL` and `WALLET_MNEMONIC` through the shell environment or a secret manager; do not put credentials in the command line. The SQLite store bootstrap requires `WALLET_MNEMONIC`; PostgreSQL also requires `DATABASE_URL`.
- To run the SQLite store bootstrap, mount `config/sqlite.toml` explicitly and persist its working directory for `crudo-store.db`.
- The mounted store bootstraps seed demo-only `admin` / `admin` and self-service demo credit; change or remove the account and do not expose top-ups.
- Wallet and ALTCHA environment variables are required only by configurations that enable their respective feature.
