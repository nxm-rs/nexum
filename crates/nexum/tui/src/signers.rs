use std::{path::PathBuf, time::Instant};

use alloy::{
    dyn_abi::TypedData,
    primitives::{Address, B256},
    signers::{
        k256::ecdsa::SigningKey,
        ledger::{HDPath, LedgerSigner},
        local::LocalSigner,
        Signature, Signer, SignerSync,
    },
};
use eyre::OptionExt;

#[derive(Debug, Clone)]
pub struct NexumAccount {
    name: String,
    signer: NexumSigner,
}

impl NexumAccount {
    pub fn is_locked(&self) -> bool {
        self.signer.is_locked()
    }

    pub fn try_unlock(&mut self, password: String) -> eyre::Result<()> {
        match &mut self.signer {
            NexumSigner::Keystore(path, signer) => {
                if signer.is_none() {
                    let keystore = LocalSigner::<SigningKey>::decrypt_keystore(path, password)?;
                    *signer = Some(keystore);
                }
                Ok(())
            }
            NexumSigner::Ledger(_, _) => Ok(()),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn address(&self) -> Option<Address> {
        self.signer.address()
    }

    pub async fn sign_hash(&self, hash: &B256) -> eyre::Result<Signature> {
        self.signer.sign_hash(hash).await
    }

    pub async fn sign_message(&self, message: &[u8]) -> eyre::Result<Signature> {
        self.signer.sign_message(message).await
    }

    pub async fn sign_dynamic_typed_data(&self, payload: &TypedData) -> eyre::Result<Signature> {
        self.signer.sign_dynamic_typed_data(payload).await
    }
}

#[derive(Debug, Clone)]
pub enum NexumSigner {
    Keystore(PathBuf, Option<LocalSigner<SigningKey>>),
    Ledger(HDPath, Address),
}

impl NexumSigner {
    fn is_locked(&self) -> bool {
        match self {
            NexumSigner::Keystore(_, signer) => signer.is_none(),
            // TODO: can probably check some method to see if the ledger is returning some
            // response, will likely make this method async, leaving for refactoring later
            NexumSigner::Ledger(_, _) => false,
        }
    }

    async fn sign_hash(&self, hash: &B256) -> eyre::Result<Signature> {
        match self {
            NexumSigner::Keystore(_, signer) => match signer {
                Some(signer) => Ok(signer.sign_hash_sync(hash)?),
                None => eyre::bail!("signer not available"),
            },
            NexumSigner::Ledger(dpath, _) => {
                let signer = LedgerSigner::new(dpath.clone(), None).await?;
                Ok(signer.sign_hash(hash).await?)
            }
        }
    }

    async fn sign_message(&self, message: &[u8]) -> eyre::Result<Signature> {
        match self {
            NexumSigner::Keystore(_, signer) => match signer {
                Some(signer) => Ok(signer.sign_message_sync(message)?),
                None => eyre::bail!("signer not available"),
            },
            NexumSigner::Ledger(dpath, _) => {
                let signer = LedgerSigner::new(dpath.clone(), None).await?;
                Ok(signer.sign_message(message).await?)
            }
        }
    }

    async fn sign_dynamic_typed_data(&self, payload: &TypedData) -> eyre::Result<Signature> {
        match self {
            NexumSigner::Keystore(_, signer) => match signer {
                Some(signer) => Ok(signer.sign_dynamic_typed_data_sync(payload)?),
                None => eyre::bail!("signer not available"),
            },
            NexumSigner::Ledger(dpath, _) => {
                let signer = LedgerSigner::new(dpath.clone(), None).await?;
                Ok(signer.sign_dynamic_typed_data(payload).await?)
            }
        }
    }

    fn address(&self) -> Option<Address> {
        match self {
            NexumSigner::Keystore(_, signer) => signer.as_ref().map(|s| s.address()),
            NexumSigner::Ledger(_, address) => Some(*address),
        }
    }

    async fn ledger(path: HDPath) -> eyre::Result<Self> {
        let signer = LedgerSigner::new(path.clone(), None).await?;
        let address = signer.get_address().await?;
        Ok(Self::Ledger(path, address))
    }
}

/// Returns all the keystore file paths in the foundry keystore directory.
pub fn load_foundry_keystores() -> eyre::Result<Vec<NexumAccount>> {
    let home_dir = std::env::home_dir().ok_or_eyre("home directory not found")?;
    let foundry_keystore_dir = home_dir.join(".foundry/keystores");
    let foundry_keystore_files = std::fs::read_dir(foundry_keystore_dir)?;

    Ok(foundry_keystore_files
        .into_iter()
        .filter_map(|f| {
            f.ok().map(|f| NexumAccount {
                name: f.file_name().to_string_lossy().to_string(),
                signer: NexumSigner::Keystore(f.path(), None),
            })
        })
        .collect::<Vec<_>>())
}

/// Returns first n ledger accounts
pub async fn load_ledger_accounts(n: usize) -> eyre::Result<Vec<NexumAccount>> {
    let start = Instant::now();
    tracing::debug!("starting loading ledger accounts");
    let mut accounts = Vec::with_capacity(n);
    for i in 0..n {
        let path = HDPath::LedgerLive(i);
        let signer = NexumSigner::ledger(path).await?;
        accounts.push(NexumAccount {
            name: format!("Ledger #{i}"),
            signer,
        });
    }
    tracing::debug!(
        elapsed = start.elapsed().as_millis(),
        "loading {n} ledger accounts"
    );
    Ok(accounts)
}
