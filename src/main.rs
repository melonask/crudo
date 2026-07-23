use std::path::Path;

use anyhow::{Context, Result, bail};
use clap::Parser;
use crudo::{Config, connect, load_config, run};

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Local configuration path or HTTPS URL. Omit to use ./Crudo.toml.
    #[arg(long, value_name = "PATH_OR_HTTPS_URL")]
    config: Option<String>,

    /// Override the configured listen address
    #[arg(long)]
    address: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut config = load_startup_config(cli.config.as_deref(), Path::new("./Crudo.toml")).await?;
    if let Some(address) = cli.address {
        config.set_address(address);
    }
    let pool = connect(&config).await?;
    run(pool, config).await
}

async fn load_startup_config(location: Option<&str>, conventional_path: &Path) -> Result<Config> {
    match location {
        Some(location) => load_config(location).await,
        None => {
            if tokio::fs::try_exists(conventional_path)
                .await
                .with_context(|| {
                    format!(
                        "could not inspect conventional configuration {}",
                        conventional_path.display()
                    )
                })?
            {
                load_config(&conventional_path.to_string_lossy()).await
            } else {
                bail!(
                    "configuration not found at {}; create ./Crudo.toml or pass --config PATH_OR_HTTPS_URL",
                    conventional_path.display()
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn omitted_config_requires_conventional_configuration() {
        let temporary_directory = tempfile::tempdir().unwrap();
        let conventional_path = temporary_directory.path().join("Crudo.toml");

        let error = match load_startup_config(None, &conventional_path).await {
            Ok(_) => panic!("missing conventional configuration was accepted"),
            Err(error) => error,
        };

        assert!(error.to_string().contains("create ./Crudo.toml"));
        assert!(error.to_string().contains("pass --config"));
    }

    #[tokio::test]
    async fn omitted_config_uses_conventional_configuration_when_present() {
        let temporary_directory = tempfile::tempdir().unwrap();
        let conventional_path = temporary_directory.path().join("Crudo.toml");
        tokio::fs::write(
            &conventional_path,
            r#"
            [database]
            url = "${CRUDO_TEST_CONVENTIONAL_CONFIG_7FC49D3A}"
            "#,
        )
        .await
        .unwrap();

        let error = match load_startup_config(None, &conventional_path).await {
            Ok(_) => panic!("conventional configuration was not selected"),
            Err(error) => error,
        };
        assert!(
            error
                .to_string()
                .contains("required environment variable CRUDO_TEST_CONVENTIONAL_CONFIG_7FC49D3A")
        );
    }

    #[tokio::test]
    async fn invalid_conventional_configuration_does_not_fall_back() {
        let temporary_directory = tempfile::tempdir().unwrap();
        let conventional_path = temporary_directory.path().join("Crudo.toml");
        tokio::fs::write(&conventional_path, "[server]\nunexpected = true")
            .await
            .unwrap();

        let error = match load_startup_config(None, &conventional_path).await {
            Ok(_) => panic!("invalid conventional configuration was accepted"),
            Err(error) => error,
        };
        let message = format!("{error:#}");
        assert!(message.contains("invalid configuration"));
        assert!(message.contains("unexpected"));
    }

    #[tokio::test]
    async fn explicit_config_delegates_to_load_config() {
        let temporary_directory = tempfile::tempdir().unwrap();
        let explicit_path = temporary_directory.path().join("explicit.toml");

        let error =
            match load_startup_config(Some(&explicit_path.to_string_lossy()), Path::new("ignored"))
                .await
            {
                Ok(_) => panic!("missing explicit configuration was accepted"),
                Err(error) => error,
            };

        assert!(
            error
                .to_string()
                .contains(&format!("could not read {}", explicit_path.display()))
        );
    }
}
