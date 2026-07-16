# Public Rust API

The crate re-exports these public items:

| Item | Purpose |
|---|---|
| `Config` | Parsed application configuration. `Config::parse(&str) -> Result<Config>` expands `${VAR}` then parses TOML. `set_address(String)` changes the listener address. |
| `load_config(&str) -> Result<Config>` | Asynchronously reads local or HTTPS configuration, expands environment, and parses it; HTTP URLs are rejected. |
| `connect(&Config) -> Result<AnyPool>` | Installs SQL drivers and connects to `database.url`. |
| `prepare_database(&AnyPool, &Config) -> Result<()>` | Runs `database.setup` atomically. |
| `build_router(AnyPool, Config) -> Result<axum::Router>` | Validates configuration and builds routes; does not prepare schema or bind a socket. |
| `run(AnyPool, Config) -> Result<()>` | Validates/builds the router, then prepares schema, binds the configured address, and serves until Ctrl-C/SIGTERM. |
| `serve(TcpListener, axum::Router) -> Result<()>` | Serves an already-built router with socket connect info and graceful shutdown. |

Library consumers should call `connect`, then `run`, or call `prepare_database` and `build_router` when hosting a router themselves. Configuration internals are intentionally not public fields.
