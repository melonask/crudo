# SQLite example

`config/sqlite.toml` is a local digital-store bootstrap. It requires `WALLET_MNEMONIC`, `ALTCHA_SECRET`, and `ALTCHA_KEY_SECRET`, uses `?` placeholders and `unixepoch()`, explicitly sets `prefix = "v1"`, and creates `crudo-store.db` in the working directory.

## Run it

In one terminal, set a BIP-39 mnemonic and run the API from the repository root:

```sh
read -rs 'WALLET_MNEMONIC?Wallet mnemonic: '
export WALLET_MNEMONIC
read -rs 'ALTCHA_SECRET?ALTCHA signing secret: '
export ALTCHA_SECRET
read -rs 'ALTCHA_KEY_SECRET?ALTCHA key-signing secret: '
export ALTCHA_KEY_SECRET
crudo --config config/sqlite.toml
```

Open the [live store demo](https://demo-crudo.github.io/). Its visible **API URL** field accepts any compatible API base and defaults exactly to `http://127.0.0.1:3000/v1`. This SQLite API explicitly sets `prefix = "v1"`.

The shipped CORS configuration permits the hosted UI at `https://demo-crudo.github.io` plus local development at `http://127.0.0.1:8000` and `http://localhost:8000`. Custom deployments must configure their own exact origins; use existing `${ENV}` expansion in TOML when appropriate rather than changing Rust. Independently of CORS, an HTTPS page targeting plain HTTP localhost may be blocked by browser local-network or mixed-content policy.

For local frontend development or testing against this HTTP API, clone [demo-crudo/demo-crudo.github.io](https://github.com/demo-crudo/demo-crudo.github.io) and serve that repository on `127.0.0.1:8000` or `localhost:8000`. Crudo does not serve or hardcode the frontend.

Startup idempotently adds four products and the demo-only `admin` / `admin` administrator when absent; it does not overwrite edited rows. Each registration derives and persists Base and Solana user wallets. Registration and login require a fresh, one-time, IP-bound ALTCHA proof from `GET /v1/challenge`. `GET /v1/payment-methods` is the database-driven frontend source for active supported coins and blockchains. An insufficient purchase balance returns the configured x402 `402` requirement with GLOBAL exact Base USDC acceptance and user-specific deposit destinations. Crudo does not verify or settle x402 payments; the self-service top-up endpoint is development-only demo credit and must not be exposed.

The bootstrap demonstrates schema setup and store flows; it is not migration tooling.
