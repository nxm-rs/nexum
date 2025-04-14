use std::sync::Arc;

use alloy_consensus::SignableTransaction;
use alloy_network::{AnyNetwork, EthereumWallet, IntoWallet};
use alloy_primitives::{Address, B256, ChainId, Signature, address};
use alloy_signer::{Result, Signer, sign_transaction_with_chain_id};
use async_trait::async_trait;
use nexum_apdu_core::{ApduExecutorErrors, Executor, SecureChannelExecutor};
use nexum_keycard::{Error, KeyPath, Keycard};
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct KeycardSigner<E>
where
    E: Executor + SecureChannelExecutor,
    Error: From<<E as ApduExecutorErrors>::Error>,
{
    inner: Arc<Mutex<Keycard<E>>>,
    pub(crate) chain_id: Option<ChainId>,
    pub(crate) address: Address,
}

impl<E> KeycardSigner<E>
where
    E: Executor + SecureChannelExecutor,
    Error: From<<E as ApduExecutorErrors>::Error>,
{
    pub fn new(keycard: Arc<Mutex<Keycard<E>>>) -> Self {
        let address = address!("0xf888b1c80d40c08e53e4f3446ae2dac72fe0f31c");
        Self {
            inner: keycard,
            chain_id: None,
            address,
        }
    }
}

#[cfg_attr(not(target_family = "wasm"), async_trait)]
impl<E> Signer for KeycardSigner<E>
where
    E: Executor + SecureChannelExecutor,
    Error: From<<E as ApduExecutorErrors>::Error>,
{
    #[inline]
    async fn sign_hash(&self, data: &B256) -> Result<Signature> {
        self.inner
            .lock()
            .await
            .sign(data, &KeyPath::Current)
            .map_err(|e| alloy_signer::Error::Other(Box::new(e)))
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
impl<E> alloy_network::TxSigner<Signature> for KeycardSigner<E>
where
    E: Executor + SecureChannelExecutor,
    Error: From<<E as ApduExecutorErrors>::Error>,
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

impl<E> IntoWallet for KeycardSigner<E>
where
    E: Executor + SecureChannelExecutor + 'static,
    Error: From<<E as ApduExecutorErrors>::Error>,
{
    type NetworkWallet = EthereumWallet;

    fn into_wallet(self) -> Self::NetworkWallet {
        EthereumWallet::from(self)
    }
}

impl<E> IntoWallet<AnyNetwork> for KeycardSigner<E>
where
    E: Executor + SecureChannelExecutor + 'static,
    Error: From<<E as ApduExecutorErrors>::Error>,
{
    type NetworkWallet = EthereumWallet;

    fn into_wallet(self) -> Self::NetworkWallet {
        EthereumWallet::from(self)
    }
}
