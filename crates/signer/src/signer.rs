use std::sync::Arc;

use alloy_consensus::SignableTransaction;
use alloy_network::{AnyNetwork, EthereumWallet, IntoWallet};
use alloy_primitives::{Address, B256, ChainId, Signature};
use alloy_signer::{Result, Signer, sign_transaction_with_chain_id};
use async_trait::async_trait;
use coins_bip32::path::DerivationPath;
use nexum_apdu_core::prelude::*;
use nexum_keycard::{ExportOption, Keycard, KeycardSecureChannel};
use tokio::sync::Mutex;

// Temporary remove Debug derive since Keycard doesn't implement Debug
pub struct KeycardSigner<T>
where
    T: CardTransport,
{
    inner: Arc<Mutex<Keycard<CardExecutor<KeycardSecureChannel<T>>>>>,
    pub(crate) chain_id: Option<ChainId>,
    pub(crate) address: Address,
    pub(crate) derivation_path: DerivationPath,
}

impl<T> std::fmt::Debug for KeycardSigner<T>
where
    T: CardTransport,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeycardSigner")
            .field("chain_id", &self.chain_id)
            .field("derivation_path", &self.derivation_path.derivation_string())
            .field("address", &self.address)
            .finish_non_exhaustive()
    }
}

impl<T> KeycardSigner<T>
where
    T: CardTransport,
{
    pub async fn new(
        keycard: Arc<Mutex<Keycard<CardExecutor<KeycardSecureChannel<T>>>>>,
        path: DerivationPath,
    ) -> Result<Self> {
        let address = KeycardSigner::address_helper(keycard.clone(), &path).await?;

        Ok(Self {
            inner: keycard,
            chain_id: None,
            address,
            derivation_path: path,
        })
    }

    pub async fn set_derivation_path(&mut self, path: DerivationPath) -> Result<()> {
        self.address = KeycardSigner::address_helper(self.inner.clone(), &path).await?;
        self.derivation_path = path;

        Ok(())
    }

    async fn address_helper(
        keycard: Arc<Mutex<Keycard<CardExecutor<KeycardSecureChannel<T>>>>>,
        path: &DerivationPath,
    ) -> Result<Address> {
        let address = keycard
            .lock()
            .await
            .export_key(ExportOption::PublicKeyOnly, path)
            .map_err(|e| alloy_signer::Error::Other(Box::new(e)))?;
        Ok(Address::from_public_key(
            &address
                .public_key()
                .ok_or(alloy_signer::Error::message("Unable to parse public key"))?
                .into(),
        ))
    }
}

#[cfg_attr(not(target_family = "wasm"), async_trait)]
impl<T> Signer for KeycardSigner<T>
where
    T: CardTransport,
{
    #[inline]
    async fn sign_hash(&self, data: &B256) -> Result<Signature> {
        // Convert the B256 to a byte slice for KeyCard's sign method
        let data_bytes: &[u8] = data.as_slice();

        // Get the keycard signature
        let signature = self
            .inner
            .lock()
            .await
            .sign(data_bytes, &self.derivation_path, false)
            .map_err(|e| alloy_signer::Error::Other(Box::new(e)))?;

        Ok(signature)
    }

    #[inline]
    fn address(&self) -> Address {
        self.address
    }

    #[inline]
    fn chain_id(&self) -> Option<ChainId> {
        self.chain_id
    }

    #[inline]
    fn set_chain_id(&mut self, chain_id: Option<ChainId>) {
        self.chain_id = chain_id;
    }
}

#[cfg_attr(not(target_family = "wasm"), async_trait)]
impl<T> alloy_network::TxSigner<Signature> for KeycardSigner<T>
where
    T: CardTransport,
{
    fn address(&self) -> Address {
        self.address
    }

    #[inline]
    async fn sign_transaction(
        &self,
        tx: &mut dyn SignableTransaction<Signature>,
    ) -> Result<Signature> {
        sign_transaction_with_chain_id!(self, tx, self.sign_hash(&tx.signature_hash()).await)
    }
}

impl<T> IntoWallet for KeycardSigner<T>
where
    T: CardTransport + 'static,
{
    type NetworkWallet = EthereumWallet;

    fn into_wallet(self) -> Self::NetworkWallet {
        EthereumWallet::from(self)
    }
}

impl<T> IntoWallet<AnyNetwork> for KeycardSigner<T>
where
    T: CardTransport + 'static,
{
    type NetworkWallet = EthereumWallet;

    fn into_wallet(self) -> Self::NetworkWallet {
        EthereumWallet::from(self)
    }
}
