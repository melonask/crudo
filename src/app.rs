use anyhow::Result;
use sqlx::AnyPool;

use crate::{Config, build_router, database::prepare_database_setup};

/// Builds and serves the configured application until an OS shutdown signal arrives.
pub async fn run(pool: AnyPool, config: Config) -> Result<()> {
    let address = config.server.address.clone();
    let setup = config.database.setup.clone();
    let app = build_router(pool.clone(), config)?;
    prepare_database_setup(&pool, &setup).await?;
    let listener = tokio::net::TcpListener::bind(&address).await?;
    serve(listener, app).await
}

/// Serves an already-built router. Exposed so integration tests can use a real TCP socket.
pub async fn serve(listener: tokio::net::TcpListener, app: axum::Router) -> Result<()> {
    let address = listener.local_addr()?;
    println!("Listening on http://{address}");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;
    Ok(())
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("could not install SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = terminate.recv() => {}
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

#[cfg(test)]
mod tests {
    use sqlx::any::AnyPoolOptions;

    use super::*;

    #[tokio::test]
    async fn invalid_router_configuration_does_not_run_database_setup() {
        sqlx::any::install_default_drivers();
        let config = Config::parse(
            r#"
            [database]
            url = "sqlite::memory:"
            setup = ["CREATE TABLE should_not_exist (id INTEGER PRIMARY KEY)"]

            [[endpoints]]
            method = "GET"
            path = "/items"
            action = "items"

            [actions.items]
            sql = "SELECT 1 AS id"
            status = 404
            "#,
        )
        .unwrap();
        let pool = AnyPoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        assert!(run(pool.clone(), config).await.is_err());
        let table_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'should_not_exist'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(table_count, 0);
    }
}
