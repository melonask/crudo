use std::{collections::HashMap, str::FromStr};

use anyhow::{Context, Result, bail};
use bip39::Mnemonic;
use bitcoin::{
    Address, CompressedPublicKey, Network,
    bip32::{DerivationPath, Xpriv},
    secp256k1::{PublicKey as SecpPublicKey, Secp256k1},
};
use ed25519_dalek::SigningKey;
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha512;
use sha3::{Digest, Keccak256};
use zeroize::{Zeroize, Zeroizing};

use crate::config::{
    BitcoinNetwork, WalletAddressFormat, WalletCurve, WalletDerivation, WalletProfile, Wallets,
};

pub(crate) struct WalletGenerator {
    seed: Zeroizing<[u8; 64]>,
    profiles: HashMap<String, WalletProfile>,
}

pub(crate) struct GeneratedAddress {
    pub(crate) address: String,
    pub(crate) derivation_path: String,
}

impl WalletGenerator {
    pub(crate) fn new(config: Wallets) -> Result<Self> {
        let phrase = Zeroizing::new(config.mnemonic);
        let passphrase = Zeroizing::new(config.passphrase);
        let mnemonic = Mnemonic::parse(phrase.as_str()).context("wallet mnemonic is invalid")?;
        if config.profiles.is_empty() {
            bail!("wallets.profiles must contain at least one profile");
        }

        let mut profiles = HashMap::new();
        for profile in config.profiles {
            validate_profile(&profile)?;
            let name = profile.name.clone();
            if profiles.insert(name.clone(), profile).is_some() {
                bail!("duplicate wallet profile {name}");
            }
        }

        Ok(Self {
            seed: Zeroizing::new(mnemonic.to_seed(passphrase.as_str())),
            profiles,
        })
    }

    pub(crate) fn profile(&self, name: &str) -> Option<&WalletProfile> {
        self.profiles.get(name)
    }

    pub(crate) fn profiles(&self) -> impl Iterator<Item = &WalletProfile> {
        self.profiles.values()
    }

    pub(crate) fn derive(
        &self,
        profile: &WalletProfile,
        values: &HashMap<String, u32>,
    ) -> Result<GeneratedAddress> {
        let path = render_path(&profile.path, values)?;
        let address = match profile.address_format {
            WalletAddressFormat::Evm => self.derive_evm(&path)?,
            WalletAddressFormat::P2wpkh => self.derive_p2wpkh(profile, &path)?,
            WalletAddressFormat::Base58PublicKey => self.derive_solana(&path)?,
        };
        Ok(GeneratedAddress {
            address,
            derivation_path: path,
        })
    }

    fn derive_secp256k1(&self, path: &str) -> Result<SecpPublicKey> {
        let path = DerivationPath::from_str(path).context("invalid BIP-32 derivation path")?;
        let root = Xpriv::new_master(Network::Bitcoin, self.seed.as_ref())?;
        let child = root.derive_priv(&Secp256k1::new(), &path)?;
        Ok(SecpPublicKey::from_secret_key(
            &Secp256k1::new(),
            &child.private_key,
        ))
    }

    fn derive_evm(&self, path: &str) -> Result<String> {
        let public_key = self.derive_secp256k1(path)?.serialize_uncompressed();
        let hash = Keccak256::digest(&public_key[1..]);
        let lowercase = hex(&hash[12..]);
        let checksum = Keccak256::digest(lowercase.as_bytes());
        let mut address = String::with_capacity(42);
        address.push_str("0x");
        for (position, character) in lowercase.bytes().enumerate() {
            let nibble = if position % 2 == 0 {
                checksum[position / 2] >> 4
            } else {
                checksum[position / 2] & 0x0f
            };
            if character.is_ascii_alphabetic() && nibble >= 8 {
                address.push((character as char).to_ascii_uppercase());
            } else {
                address.push(character as char);
            }
        }
        Ok(address)
    }

    fn derive_p2wpkh(&self, profile: &WalletProfile, path: &str) -> Result<String> {
        let public_key = CompressedPublicKey(self.derive_secp256k1(path)?);
        let network = match profile.network.context("P2WPKH profile requires network")? {
            BitcoinNetwork::Mainnet => Network::Bitcoin,
            BitcoinNetwork::Testnet => Network::Testnet,
            BitcoinNetwork::Signet => Network::Signet,
            BitcoinNetwork::Regtest => Network::Regtest,
        };
        Ok(Address::p2wpkh(&public_key, network).to_string())
    }

    fn derive_solana(&self, path: &str) -> Result<String> {
        let mut key = [0_u8; 32];
        let mut chain_code = [0_u8; 32];
        let master = hmac_sha512(b"ed25519 seed", self.seed.as_ref())?;
        key.copy_from_slice(&master[..32]);
        chain_code.copy_from_slice(&master[32..]);

        for index in parse_slip10_path(path)? {
            let mut data = [0_u8; 37];
            data[1..33].copy_from_slice(&key);
            data[33..].copy_from_slice(&(index | (1 << 31)).to_be_bytes());
            let child = hmac_sha512(&chain_code, &data)?;
            data.zeroize();
            key.copy_from_slice(&child[..32]);
            chain_code.copy_from_slice(&child[32..]);
        }

        let address =
            bs58::encode(SigningKey::from_bytes(&key).verifying_key().as_bytes()).into_string();
        key.zeroize();
        chain_code.zeroize();
        Ok(address)
    }
}

fn validate_profile(profile: &WalletProfile) -> Result<()> {
    if profile.name.is_empty() || profile.name.chars().any(char::is_whitespace) {
        bail!("wallet profile name must be non-empty and contain no whitespace");
    }
    let Some((namespace, reference)) = profile.caip2.split_once(':') else {
        bail!("wallet profile {} has invalid caip2", profile.name);
    };
    if namespace.len() < 3
        || namespace.len() > 8
        || !namespace
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        || reference.is_empty()
        || reference.len() > 32
        || !reference
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        bail!("wallet profile {} has invalid caip2", profile.name);
    }
    if profile.max_addresses == 0 || profile.max_addresses > 1 << 31 {
        bail!("wallet profile {} has invalid max_addresses", profile.name);
    }
    let placeholders = path_placeholders(&profile.path)?;
    if placeholders.is_empty() {
        bail!("wallet profile {} path has no placeholders", profile.name);
    }
    let test_path = render_path(
        &profile.path,
        &placeholders.into_iter().map(|name| (name, 0)).collect(),
    )?;
    match (profile.curve, profile.derivation, profile.address_format) {
        (WalletCurve::Secp256k1, WalletDerivation::Bip32, WalletAddressFormat::Evm) => {
            if profile.network.is_some() {
                bail!(
                    "wallet profile {} network is only valid for P2WPKH",
                    profile.name
                );
            }
            DerivationPath::from_str(&test_path)
                .with_context(|| format!("wallet profile {} has invalid path", profile.name))?;
        }
        (WalletCurve::Secp256k1, WalletDerivation::Bip32, WalletAddressFormat::P2wpkh) => {
            if profile.network.is_none() {
                bail!("wallet profile {} requires network", profile.name);
            }
            DerivationPath::from_str(&test_path)
                .with_context(|| format!("wallet profile {} has invalid path", profile.name))?;
        }
        (WalletCurve::Ed25519, WalletDerivation::Slip10, WalletAddressFormat::Base58PublicKey) => {
            if profile.network.is_some() {
                bail!(
                    "wallet profile {} network is only valid for P2WPKH",
                    profile.name
                );
            }
            parse_slip10_path(&test_path)
                .with_context(|| format!("wallet profile {} has invalid path", profile.name))?;
        }
        _ => bail!(
            "wallet profile {} has an incompatible curve, derivation, and address format",
            profile.name
        ),
    }
    Ok(())
}

pub(crate) fn path_placeholders(path: &str) -> Result<Vec<String>> {
    let mut placeholders = Vec::new();
    let mut rest = path;
    while let Some(start) = rest.find('{') {
        if rest[..start].contains('}') {
            bail!("unmatched wallet path placeholder terminator");
        }
        let after = &rest[start + 1..];
        let end = after
            .find('}')
            .context("unclosed wallet path placeholder")?;
        let name = &after[..end];
        if name.is_empty()
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        {
            bail!("invalid wallet path placeholder {name}");
        }
        if placeholders.iter().any(|existing| existing == name) {
            bail!("duplicate wallet path placeholder {name}");
        }
        placeholders.push(name.to_owned());
        rest = &after[end + 1..];
    }
    if rest.contains('}') {
        bail!("unmatched wallet path placeholder terminator");
    }
    Ok(placeholders)
}

fn render_path(path: &str, values: &HashMap<String, u32>) -> Result<String> {
    let placeholders = path_placeholders(path)?;
    if placeholders.len() != values.len()
        || placeholders.iter().any(|name| !values.contains_key(name))
    {
        bail!("wallet path values do not match its placeholders");
    }
    let mut rendered = path.to_owned();
    for name in placeholders {
        let value = values[&name];
        if value >= 1 << 31 {
            bail!("wallet path value {name} must be less than 2^31");
        }
        rendered = rendered.replace(&format!("{{{name}}}"), &value.to_string());
    }
    Ok(rendered)
}

fn parse_slip10_path(path: &str) -> Result<Vec<u32>> {
    let Some(path) = path.strip_prefix("m/") else {
        bail!("SLIP-10 path must start with m/");
    };
    if path.is_empty() {
        bail!("SLIP-10 path must contain at least one child");
    }
    path.split('/')
        .map(|child| {
            let value = child
                .strip_suffix('\'')
                .context("SLIP-10 ed25519 children must be hardened")?
                .parse::<u32>()
                .context("invalid SLIP-10 child index")?;
            if value >= 1 << 31 {
                bail!("SLIP-10 child index must be less than 2^31");
            }
            Ok(value)
        })
        .collect()
}

fn hmac_sha512(key: &[u8], data: &[u8]) -> Result<Zeroizing<[u8; 64]>> {
    let mut mac = Hmac::<Sha512>::new_from_slice(key).context("invalid HMAC key")?;
    mac.update(data);
    Ok(Zeroizing::new(mac.finalize().into_bytes().into()))
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut value = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        value.push(DIGITS[(byte >> 4) as usize] as char);
        value.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn generator() -> WalletGenerator {
        let config = Config::parse(
            r#"
            [database]
            url = "sqlite::memory:"

            [wallets]
            mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"

            [[wallets.profiles]]
            name = "ethereum-mainnet"
            caip2 = "eip155:1"
            curve = "secp256k1"
            derivation = "bip32"
            path = "m/44'/60'/{user_id}'/0/{address_index}"
            address_format = "evm"
            max_addresses = 5

            [[wallets.profiles]]
            name = "solana-mainnet"
            caip2 = "solana:mainnet"
            curve = "ed25519"
            derivation = "slip10"
            path = "m/44'/501'/{user_id}'/{address_index}'"
            address_format = "base58-public-key"
            max_addresses = 5

            [[wallets.profiles]]
            name = "bitcoin-mainnet"
            caip2 = "bip122:mainnet"
            curve = "secp256k1"
            derivation = "bip32"
            path = "m/84'/0'/{user_id}'/0/{address_index}"
            address_format = "p2wpkh"
            network = "mainnet"
            max_addresses = 5
            "#,
        )
        .unwrap();
        WalletGenerator::new(config.wallets.unwrap()).unwrap()
    }

    #[test]
    fn derives_known_addresses_for_supported_formats() {
        let generator = generator();
        let values = HashMap::from([("user_id".into(), 0), ("address_index".into(), 0)]);

        let evm = generator
            .derive(generator.profile("ethereum-mainnet").unwrap(), &values)
            .unwrap();
        let solana = generator
            .derive(generator.profile("solana-mainnet").unwrap(), &values)
            .unwrap();
        let bitcoin = generator
            .derive(generator.profile("bitcoin-mainnet").unwrap(), &values)
            .unwrap();

        assert_eq!(evm.address, "0x9858EfFD232B4033E47d90003D41EC34EcaEda94");
        assert_eq!(
            solana.address,
            "HAgk14JpMQLgt6rVgv7cBQFJWFto5Dqxi472uT3DKpqk"
        );
        assert_eq!(
            bitcoin.address,
            "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu"
        );
        assert_eq!(evm.derivation_path, "m/44'/60'/0'/0/0");
        assert_eq!(solana.derivation_path, "m/44'/501'/0'/0'");
    }

    #[test]
    fn index_changes_every_derived_address() {
        let generator = generator();
        for name in ["ethereum-mainnet", "solana-mainnet", "bitcoin-mainnet"] {
            let profile = generator.profile(name).unwrap();
            let first = HashMap::from([("user_id".into(), 1), ("address_index".into(), 1)]);
            let second = HashMap::from([("user_id".into(), 1), ("address_index".into(), 2)]);
            assert_ne!(
                generator.derive(profile, &first).unwrap().address,
                generator.derive(profile, &second).unwrap().address
            );
        }
    }

    #[test]
    fn rejects_unsupported_profile_combinations_and_indices() {
        let config = Config::parse(
            r#"
            [database]
            url = "sqlite::memory:"

            [wallets]
            mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"

            [[wallets.profiles]]
            name = "invalid"
            caip2 = "eip155:1"
            curve = "ed25519"
            derivation = "bip32"
            path = "m/44'/{user_id}'"
            address_format = "evm"
            max_addresses = 5
            "#,
        )
        .unwrap();
        assert!(WalletGenerator::new(config.wallets.unwrap()).is_err());

        let generator = generator();
        let values = HashMap::from([("user_id".into(), 1 << 31), ("address_index".into(), 0)]);
        assert!(
            generator
                .derive(generator.profile("solana-mainnet").unwrap(), &values)
                .is_err()
        );
    }
}
