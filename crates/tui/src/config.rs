use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
};

use alloy::primitives::Address;
use alloy_chains::NamedChain;
use eyre::OptionExt;
use figment::{
    providers::{Format, Toml},
    Figment,
};
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub rpcs: HashMap<String, Url>,
    pub origin_connections: BTreeMap<Address, HashMap<Url, bool>>,
    pub labels: BTreeMap<NamedChain, HashMap<Address, String>>,
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

pub fn load_config() -> eyre::Result<Config> {
    Ok(Figment::new()
        .merge(Toml::file(config_dir()?.join("nxm.toml")))
        .extract()?)
}
