use anyhow::Result;
use clap::Parser;
use crudo::{connect, load_config, run};

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[arg(long, default_value = "config/minimal.toml")]
    config: String,

    /// Override the configured listen address
    #[arg(long)]
    address: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut config = load_config(&cli.config).await?;
    if let Some(address) = cli.address {
        config.set_address(address);
    }
    let pool = connect(&config).await?;
    run(pool, config).await
}
