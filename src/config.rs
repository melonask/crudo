use std::collections::HashMap;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub(crate) database: Database,
    #[serde(default)]
    pub(crate) server: Server,
    #[serde(default)]
    pub(crate) endpoints: Vec<Endpoint>,
    #[serde(default)]
    pub(crate) actions: HashMap<String, Action>,
    #[serde(default)]
    pub(crate) auth: Authentication,
    pub(crate) altcha: Option<Altcha>,
    pub(crate) wallets: Option<Wallets>,
}

impl Config {
    pub fn parse(source: &str) -> Result<Self> {
        let source = expand_env(source)?;
        toml::from_str(&source).context("invalid configuration")
    }

    pub fn set_address(&mut self, address: String) {
        self.server.address = address;
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Database {
    pub(crate) url: String,
    #[serde(default)]
    pub(crate) setup: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Server {
    #[serde(default = "default_address")]
    pub(crate) address: String,
    #[serde(default)]
    pub(crate) prefix: String,
    #[serde(default)]
    pub(crate) limits: Limits,
    pub(crate) cors: Option<Cors>,
}

impl Default for Server {
    fn default() -> Self {
        Self {
            address: default_address(),
            prefix: String::new(),
            limits: Limits::default(),
            cors: None,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Cors {
    pub(crate) origins: Vec<String>,
}

fn default_address() -> String {
    "127.0.0.1:3000".into()
}

#[derive(Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Limits {
    #[serde(default = "default_body_bytes")]
    pub(crate) body_bytes: usize,
    #[serde(default = "default_timeout_seconds")]
    pub(crate) timeout_seconds: u64,
    #[serde(default = "default_concurrency")]
    pub(crate) concurrency: usize,
    #[serde(default = "default_requests")]
    pub(crate) requests: u32,
    #[serde(default = "default_window_seconds")]
    pub(crate) window_seconds: u64,
}

impl Limits {
    pub(crate) fn with_overrides(self, overrides: EndpointLimits) -> Self {
        Self {
            body_bytes: overrides.body_bytes.unwrap_or(self.body_bytes),
            timeout_seconds: overrides.timeout_seconds.unwrap_or(self.timeout_seconds),
            concurrency: overrides.concurrency.unwrap_or(self.concurrency),
            requests: overrides.requests.unwrap_or(self.requests),
            window_seconds: overrides.window_seconds.unwrap_or(self.window_seconds),
        }
    }
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            body_bytes: default_body_bytes(),
            timeout_seconds: default_timeout_seconds(),
            concurrency: default_concurrency(),
            requests: default_requests(),
            window_seconds: default_window_seconds(),
        }
    }
}

fn default_body_bytes() -> usize {
    1_048_576
}

fn default_timeout_seconds() -> u64 {
    30
}

fn default_concurrency() -> usize {
    100
}

fn default_requests() -> u32 {
    120
}

fn default_window_seconds() -> u64 {
    60
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Endpoint {
    pub(crate) method: String,
    pub(crate) path: String,
    pub(crate) action: String,
    #[serde(default)]
    pub(crate) auth: Vec<AuthMethod>,
    #[serde(default)]
    pub(crate) auth_optional: bool,
    #[serde(default)]
    pub(crate) altcha: bool,
    #[serde(default = "default_true")]
    pub(crate) altcha_for_authenticated: bool,
    #[serde(default)]
    pub(crate) limits: EndpointLimits,
}

#[derive(Clone, Copy, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct EndpointLimits {
    pub(crate) body_bytes: Option<usize>,
    pub(crate) timeout_seconds: Option<u64>,
    pub(crate) concurrency: Option<usize>,
    pub(crate) requests: Option<u32>,
    pub(crate) window_seconds: Option<u64>,
}

fn default_true() -> bool {
    true
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Action {
    pub(crate) sql: String,
    #[serde(default)]
    pub(crate) params: Vec<String>,
    #[serde(default)]
    pub(crate) result: ResultMode,
    #[serde(default)]
    pub(crate) hash: Vec<String>,
    #[serde(default)]
    pub(crate) no_store: bool,
    pub(crate) status: Option<u16>,
    #[serde(default)]
    pub(crate) errors: Vec<ActionError>,
    pub(crate) wallets: Option<ActionWallets>,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ActionWallets {
    #[serde(default)]
    pub(crate) profiles: Vec<String>,
    pub(crate) profile: Option<String>,
    pub(crate) values: HashMap<String, String>,
    pub(crate) sql: String,
    pub(crate) params: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Wallets {
    pub(crate) mnemonic: String,
    #[serde(default)]
    pub(crate) passphrase: String,
    #[serde(default)]
    pub(crate) profiles: Vec<WalletProfile>,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WalletProfile {
    pub(crate) name: String,
    pub(crate) caip2: String,
    pub(crate) curve: WalletCurve,
    pub(crate) derivation: WalletDerivation,
    pub(crate) path: String,
    pub(crate) address_format: WalletAddressFormat,
    pub(crate) network: Option<BitcoinNetwork>,
    pub(crate) max_addresses: u32,
}

#[derive(Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum WalletCurve {
    Secp256k1,
    Ed25519,
}

#[derive(Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum WalletDerivation {
    Bip32,
    Slip10,
}

#[derive(Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum WalletAddressFormat {
    Evm,
    Base58PublicKey,
    P2wpkh,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum BitcoinNetwork {
    Mainnet,
    Testnet,
    Signet,
    Regtest,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ActionError {
    pub(crate) database_message: String,
    pub(crate) status: u16,
    pub(crate) message: String,
}

#[derive(Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ResultMode {
    #[default]
    Execute,
    One,
    Optional,
    Many,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum AuthMethod {
    Basic,
    Bearer,
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Authentication {
    pub(crate) basic: Option<BasicAuth>,
    pub(crate) bearer: Option<BearerAuth>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BasicAuth {
    pub(crate) sql: String,
    pub(crate) owner: String,
    pub(crate) password: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BearerAuth {
    pub(crate) sql: String,
    pub(crate) owner: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Altcha {
    pub(crate) secret: String,
    pub(crate) key_secret: String,
    #[serde(default = "default_altcha_path")]
    pub(crate) path: String,
    #[serde(default = "default_altcha_algorithm")]
    pub(crate) algorithm: String,
    #[serde(default = "default_altcha_cost")]
    pub(crate) cost: u32,
    #[serde(default = "default_altcha_max_number")]
    pub(crate) max_number: u32,
    #[serde(default = "default_altcha_expires_seconds")]
    pub(crate) expires_seconds: u64,
    #[serde(default = "default_altcha_bind_ip")]
    pub(crate) bind_ip: bool,
}

fn default_altcha_path() -> String {
    "/challenge".into()
}

fn default_altcha_algorithm() -> String {
    "PBKDF2/SHA-256".into()
}

fn default_altcha_cost() -> u32 {
    5_000
}

fn default_altcha_max_number() -> u32 {
    10_000
}

fn default_altcha_expires_seconds() -> u64 {
    300
}

fn default_altcha_bind_ip() -> bool {
    true
}

pub async fn load_config(location: &str) -> Result<Config> {
    crate::tls::install_crypto_provider();
    if location.starts_with("http://") {
        bail!("remote configuration must use HTTPS: {location}");
    }
    let source = if location.starts_with("https://") {
        reqwest::get(location)
            .await
            .with_context(|| format!("could not fetch {location}"))?
            .error_for_status()
            .with_context(|| format!("could not fetch {location}"))?
            .text()
            .await?
    } else {
        tokio::fs::read_to_string(location)
            .await
            .with_context(|| format!("could not read {location}"))?
    };

    let source = expand_env(&source)?;
    toml::from_str(&source).with_context(|| format!("invalid configuration in {location}"))
}

fn expand_env(source: &str) -> Result<String> {
    let mut expanded = String::with_capacity(source.len());
    let mut rest = source;
    while let Some(start) = rest.find("${") {
        expanded.push_str(&rest[..start]);
        let variable = &rest[start + 2..];
        let end = variable
            .find('}')
            .context("unclosed environment variable")?;
        let name = &variable[..end];
        if name.is_empty() {
            bail!("environment variable name cannot be empty");
        }
        expanded.push_str(
            &std::env::var(name)
                .with_context(|| format!("required environment variable {name} is not set"))?,
        );
        rest = &variable[end + 1..];
    }
    expanded.push_str(rest);
    Ok(expanded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configuration_expands_environment_variables() {
        let path = std::env::var("PATH").unwrap();
        assert_eq!(
            expand_env("value = '${PATH}'").unwrap(),
            format!("value = '{path}'")
        );
        assert!(expand_env("${}").is_err());
        assert!(expand_env("${UNCLOSED").is_err());
    }

    #[test]
    fn configuration_rejects_unknown_fields() {
        let top_level = Config::parse(
            r#"
            [database]
            url = "sqlite::memory:"
            unexpected = true
            "#,
        )
        .err()
        .unwrap();
        assert!(format!("{top_level:#}").contains("unexpected"));

        let nested = Config::parse(
            r#"
            [database]
            url = "sqlite::memory:"

            [server.limits]
            body_btyes = 1024
            "#,
        )
        .err()
        .unwrap();
        assert!(format!("{nested:#}").contains("body_btyes"));
    }

    #[test]
    fn missing_environment_variable_explains_that_it_is_required() {
        let error = Config::parse(
            r#"
            [database]
            url = "${CRUDO_TEST_ABSENT_ENV_7FC49D3A}"
            "#,
        )
        .err()
        .unwrap();
        assert!(
            error
                .to_string()
                .contains("required environment variable CRUDO_TEST_ABSENT_ENV_7FC49D3A")
        );
    }

    #[tokio::test]
    async fn plain_http_remote_configuration_is_rejected() {
        let error = load_config("http://example.invalid/config.toml")
            .await
            .err()
            .unwrap();
        assert!(error.to_string().contains("must use HTTPS"));
    }

    #[test]
    fn minimal_configuration_needs_no_environment_variables() {
        Config::parse(include_str!("../config/minimal.toml")).unwrap();
    }
}
