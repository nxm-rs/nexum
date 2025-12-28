use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
};

use alloy::primitives::Address;
use alloy_chains::NamedChain;
use eyre::OptionExt;
use figment::{
    Figment,
    providers::{Format, Toml},
};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::signers::{NexumAccount, load_keystores};

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub rpcs: BTreeMap<String, Url>,
    #[serde(default)]
    pub origin_connections: BTreeMap<Address, HashMap<Url, bool>>,
    #[serde(default)]
    pub labels: BTreeMap<NamedChain, HashMap<Address, String>>,
    #[serde(default)]
    pub signer: SignerConfig,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct SignerConfig {
    #[serde(default)]
    pub keystores: Vec<KeystoreDir>,
    #[serde(default)]
    pub ledger: LedgerConfig,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct KeystoreDir {
    dir: String,
    ignore: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LedgerConfig {
    pub n: usize,
}

impl Default for LedgerConfig {
    fn default() -> Self {
        Self { n: 10 }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            rpcs: BTreeMap::from([
                (
                    "Mainnet".to_string(),
                    "https://eth.llamarpc.com".parse().unwrap(),
                ),
                (
                    "Gnosis".to_string(),
                    "https://rpc.gnosischain.com".parse().unwrap(),
                ),
                (
                    "Sepolia".to_string(),
                    "https://ethereum-sepolia-rpc.publicnode.com"
                        .parse()
                        .unwrap(),
                ),
                (
                    "Holesky".to_string(),
                    "https://ethereum-holesky-rpc.publicnode.com"
                        .parse()
                        .unwrap(),
                ),
                (
                    "Hoodi".to_string(),
                    "https://rpc.hoodi.ethpandaops.io".parse().unwrap(),
                ),
            ]),
            origin_connections: BTreeMap::new(),
            labels: BTreeMap::new(),
            signer: SignerConfig::default(),
        }
    }
}

impl Config {
    /// Returns chain RPCs parsed from config keys (no network validation).
    /// Invalid chain names are skipped with a warning.
    pub fn chain_rpcs(&self) -> Vec<(NamedChain, Url)> {
        self.rpcs
            .iter()
            .filter_map(|(chain_name, url)| {
                chain_name
                    .parse::<NamedChain>()
                    .map(|chain| (chain, url.clone()))
                    .inspect_err(|e| {
                        tracing::warn!(chain_name, ?e, "failed to parse chain name, skipping");
                    })
                    .ok()
            })
            .collect()
    }

    pub fn keystores(&self) -> eyre::Result<Vec<NexumAccount>> {
        Ok(self
            .signer
            .keystores
            .iter()
            .map(|k| {
                load_keystores(
                    &k.dir,
                    &k.ignore.iter().map(|x| x.as_str()).collect::<Vec<_>>()[..],
                )
            })
            .collect::<eyre::Result<Vec<_>>>()?
            .concat())
    }
}

/// Returns the base config directory for nexum. It also creates the directory
/// if it doesn't exist yet.
pub fn config_dir() -> eyre::Result<PathBuf> {
    let dir = std::env::home_dir()
        .ok_or_eyre("home directory not found")?
        .join(".nxm");
    if !dir.exists() {
        std::fs::create_dir(&dir)?
    }
    Ok(dir)
}

pub fn load_config() -> Config {
    config_dir()
        .ok()
        .and_then(|dir| {
            Figment::new()
                .merge(Toml::file(dir.join("nxm.toml")))
                .extract()
                .ok()
        })
        .unwrap_or_default()
}
