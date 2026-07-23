# PostgreSQL example

`config/postgres.toml` requires only `DATABASE_URL`. It uses `$1` placeholders, casts externally supplied numeric values where needed, and explicitly sets `prefix = "v1"`.

## Run it

Set the URL in the current shell without putting it in a command line or shell history, then start the API:

```sh
read -rs 'DATABASE_URL?PostgreSQL URL: '
export DATABASE_URL
crudo --config config/postgres.toml
```

Open the [live store demo](https://demo-crudo.github.io/). Its visible **API URL** field accepts any compatible API base and defaults exactly to `http://127.0.0.1:3000/v1`. This PostgreSQL API explicitly sets `prefix = "v1"`.

The shipped CORS configuration permits `http://127.0.0.1:8000` and `http://localhost:8000`, not the hosted GitHub Pages origin. To connect the hosted UI to an HTTPS API, explicitly add `https://demo-crudo.github.io` to `server.cors.origins`. An HTTPS page cannot normally call an HTTP API because of mixed-content restrictions.

For local frontend development or testing against an HTTP API, clone [demo-crudo/demo-crudo.github.io](https://github.com/demo-crudo/demo-crudo.github.io) and serve that repository on `127.0.0.1:8000` or `localhost:8000`. Crudo does not serve or hardcode the frontend.

Startup idempotently adds four products and demo-only `admin` / `admin` if absent and preserves subsequent edits. Change or remove that administrator before deployment. Self-service top-ups are development-only demo credit, not real payment processing, and must not be exposed. The setup is a demo bootstrap, not migration tooling. Use TLS for database transport across a trust boundary.
