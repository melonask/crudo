# SQLite example

`config/sqlite.toml` is a local digital-store bootstrap. It needs no environment variables, uses `?` placeholders and `unixepoch()`, explicitly sets `prefix = "v1"`, and creates `crudo-store.db` in the working directory.

## Run it

In one terminal, run the API from the repository root:

```sh
crudo --config config/sqlite.toml
```

Open the [live store demo](https://demo-crudo.github.io/). Its visible **API URL** field accepts any compatible API base and defaults exactly to `http://127.0.0.1:3000/v1`. This SQLite API explicitly sets `prefix = "v1"`.

The shipped CORS configuration permits `http://127.0.0.1:8000` and `http://localhost:8000`, not the hosted GitHub Pages origin. To connect the hosted UI to an HTTPS API, explicitly add `https://demo-crudo.github.io` to `server.cors.origins`. An HTTPS page cannot normally call an HTTP API because of mixed-content restrictions.

For local frontend development or testing against this HTTP API, clone [demo-crudo/demo-crudo.github.io](https://github.com/demo-crudo/demo-crudo.github.io) and serve that repository on `127.0.0.1:8000` or `localhost:8000`. Crudo does not serve or hardcode the frontend.

Startup idempotently adds four products and the demo-only `admin` / `admin` administrator when absent; it does not overwrite edited rows. Change or remove that account before any deployment. Self-service top-ups are development-only demo credit, not payment processing, and must not be exposed.

The bootstrap demonstrates schema setup and store flows; it is not migration tooling.
