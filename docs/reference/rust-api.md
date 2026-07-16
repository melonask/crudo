# Public Rust API

The crate re-exports these public items.

| Item | Purpose |
|---|---|
| `Config` | Parsed application configuration. Its fields remain private. |
| `Config::parse(&str) -> Result<Config>` | Expands `${VAR}` references and parses TOML. |
| `Config::set_address(String)` | Replaces the configured listener address. |
| `load_config(&str) -> Result<Config>` | Asynchronously reads local or HTTPS configuration, expands environment, and parses it. HTTP URLs are rejected. |
| `connect(&Config) -> Result<AnyPool>` | Installs SQL drivers and connects to `database.url`. |
| `prepare_database(&AnyPool, &Config) -> Result<()>` | Runs `database.setup` atomically. |
| `build_router(AnyPool, Config) -> Result<axum::Router>` | Validates configuration and builds routes; it does not prepare the schema or bind a socket. |
| `run(AnyPool, Config) -> Result<()>` | Validates and builds the router, prepares the schema, binds the configured address, and serves until Ctrl-C/SIGTERM. |
| `serve(TcpListener, axum::Router) -> Result<()>` | Serves an already-built router with socket connect info and graceful shutdown. |

## Recommended call sequences

### Run the complete service

1. Load or parse `Config`.
2. Call `connect(&config)`.
3. Call `run(pool, config)`.

```rust
let config = crudo::load_config("config.toml").await?;
let pool = crudo::connect(&config).await?;
crudo::run(pool, config).await?;
```

### Host the router yourself

1. Load or parse `Config`.
2. Connect, then call `prepare_database(&pool, &config)`.
3. Build the router and pass it to the host server.

```rust
let config = crudo::load_config("config.toml").await?;
let pool = crudo::connect(&config).await?;
crudo::prepare_database(&pool, &config).await?;
let router = crudo::build_router(pool, config)?;
```

## Ownership notes

- `build_router` and `run` take ownership of both `AnyPool` and `Config`.
- Prepare the database before `build_router` when hosting the router yourself.
- Configuration internals are intentionally not public fields.
