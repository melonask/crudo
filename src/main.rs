use anyhow::{Context, Result};
use clap::Parser;
use crudo::{Config, connect, load_config, run};

const MINIMAL_CONFIG: &str = include_str!("../config/minimal.toml");

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Local configuration path or HTTPS URL. Omit to use the built-in minimal starter.
    #[arg(long, value_name = "PATH_OR_HTTPS_URL")]
    config: Option<String>,

    /// Override the configured listen address
    #[arg(long)]
    address: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut config = load_startup_config(cli.config.as_deref()).await?;
    if let Some(address) = cli.address {
        config.set_address(address);
    }
    let pool = connect(&config).await?;
    run(pool, config).await
}

async fn load_startup_config(location: Option<&str>) -> Result<Config> {
    match location {
        Some(location) => load_config(location).await,
        None => {
            Config::parse(MINIMAL_CONFIG).context("could not parse built-in minimal configuration")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn omitted_config_uses_embedded_minimal_starter() {
        assert!(load_startup_config(None).await.is_ok());
    }
}
