use anyhow::Result;
use sqlx::AnyPool;

use crate::{Config, build_router, prepare_database};

/// Builds and serves the configured application until an OS shutdown signal arrives.
pub async fn run(pool: AnyPool, config: Config) -> Result<()> {
    prepare_database(&pool, &config).await?;
    let address = config.server.address.clone();
    let app = build_router(pool, config)?;
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
