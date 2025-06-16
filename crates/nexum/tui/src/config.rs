use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
};

use alloy::{
    network::Ethereum,
    primitives::Address,
    providers::{Provider, RootProvider},
};
use alloy_chains::NamedChain;
use eyre::OptionExt;
use figment::{
    providers::{Format, Toml},
    Figment,
};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::signers::{load_keystores, NexumAccount};

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub rpcs: BTreeMap<String, Url>,
    #[serde(default)]
    pub origin_connections: BTreeMap<Address, HashMap<Url, bool>>,
    #[serde(default)]
    pub labels: BTreeMap<NamedChain, HashMap<Address, String>>,
    #[serde(default)]
    pub keystores: Vec<KeystoreDir>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct KeystoreDir {
    dir: String,
    ignore: Vec<String>,
}

impl Config {
    pub async fn chain_rpcs(&self) -> eyre::Result<Vec<(NamedChain, Url)>> {
        let providers = futures::future::join_all(self.rpcs.values().map(|rpc_url| {
            let rpc_url = rpc_url.to_string();
            async move { RootProvider::<Ethereum>::connect(&rpc_url).await }
        }))
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
        let chain_ids = futures::future::join_all(providers.iter().map(|p| p.get_chain_id()))
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;
        let chains = chain_ids
            .into_iter()
            .map(NamedChain::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| eyre::eyre!("failed to parse chainid to NamedChain: {e:?}"))?;
        Ok(chains
            .into_iter()
            .zip(self.rpcs.values().cloned())
            .collect())
    }

    pub fn keystores(&self) -> eyre::Result<Vec<NexumAccount>> {
        Ok(self
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

pub fn load_config() -> eyre::Result<Config> {
    Ok(Figment::new()
        .merge(Toml::file(config_dir()?.join("nxm.toml")))
        .extract()?)
}
