use std::collections::HashMap;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

#[derive(Deserialize)]
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
pub(crate) struct Database {
    pub(crate) url: String,
    #[serde(default)]
    pub(crate) setup: Vec<String>,
}

#[derive(Deserialize)]
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
pub(crate) struct Cors {
    pub(crate) origins: Vec<String>,
}

fn default_address() -> String {
    "127.0.0.1:3000".into()
}

#[derive(Clone, Copy, Deserialize)]
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
}

#[derive(Clone, Copy, Default, Deserialize)]
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
pub(crate) struct Authentication {
    pub(crate) basic: Option<BasicAuth>,
    pub(crate) bearer: Option<BearerAuth>,
}

#[derive(Deserialize)]
pub(crate) struct BasicAuth {
    pub(crate) sql: String,
    pub(crate) owner: String,
    pub(crate) password: String,
}

#[derive(Deserialize)]
pub(crate) struct BearerAuth {
    pub(crate) sql: String,
    pub(crate) owner: String,
}

#[derive(Deserialize)]
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
    let source = if location.starts_with("http://") || location.starts_with("https://") {
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
                .with_context(|| format!("environment variable {name} is not set"))?,
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
}
