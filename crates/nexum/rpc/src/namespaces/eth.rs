use alloy::{
    consensus::EthereumTxEnvelope,
    dyn_abi::TypedData,
    network::{Ethereum, Network, NetworkWallet},
    primitives::{Address, Bytes, TxHash},
    providers::{
        Provider,
        fillers::{TxFiller, WalletFiller},
    },
    rpc::types::TransactionRequest,
};
use jsonrpsee::{
    RpcModule,
    core::RpcResult,
    types::{ErrorCode, ErrorObject},
};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};

use crate::{
    rpc::{
        GlobalRpcContext, InteractiveRequest, InteractiveResponse, json_rpc_internal_error,
        make_interactive_request,
    },
    upstream_requests,
};

pub fn init<F, P>(
    context: GlobalRpcContext<F, P>,
) -> eyre::Result<RpcModule<GlobalRpcContext<F, P>>>
where
    P: Provider + Clone + 'static,
    F: TxFiller + 'static,
{
    let mut eth_module = RpcModule::new(context);
    upstream_requests! {
        eth_module,
        "eth_syncing",
        "eth_chainId",
        "eth_gasPrice",
        "eth_blockNumber",
        "eth_getBalance",
        "eth_getStorageAt",
        "eth_getTransactionCount",
        "eth_getBlockTransactionCountByHash",
        "eth_getBlockTransactionCountByNumber",
        "eth_getUncleCountByBlockHash",
        "eth_getUncleCountByBlockNumber",
        "eth_getCode",
        "eth_sendRawTransaction",
        "eth_call",
        "eth_estimateGas",
        "eth_getBlockByHash",
        "eth_getBlockByNumber",
        "eth_getTransactionByHash",
        "eth_getTransactionByBlockHashAndIndex",
        "eth_getTransactionByBlockNumberAndIndex",
        "eth_getTransactionReceipt",
        "eth_getUncleByBlockHashAndIndex",
        "eth_getUncleByBlockNumberAndIndex",
        "eth_newFilter",
        "eth_newBlockFilter",
        "eth_newPendingTransactionFilter",
        "eth_uninstallFilter",
        "eth_getFilterChanges",
        "eth_getFilterLogs",
        "eth_getLogs",
        "eth_feeHistory"
    }

    eth_module.register_async_method(
        "eth_requestAccounts",
        async |_, ctx, _| -> RpcResult<Vec<Address>> {
            match make_interactive_request(
                ctx.sender.clone(),
                InteractiveRequest::EthRequestAccounts,
            )
            .await
            .map_err(json_rpc_internal_error)?
            {
                InteractiveResponse::EthRequestAccounts(accounts) => Ok(accounts),
                _ => Err(ErrorObject::from(ErrorCode::InternalError)),
            }
        },
    )?;

    eth_module.register_async_method(
        "eth_accounts",
        async |_, ctx, _| -> RpcResult<Vec<Address>> {
            match make_interactive_request(ctx.sender.clone(), InteractiveRequest::EthAccounts)
                .await
                .map_err(json_rpc_internal_error)?
            {
                InteractiveResponse::EthAccounts(accounts) => Ok(accounts),
                _ => Err(ErrorObject::from(ErrorCode::InternalError)),
            }
        },
    )?;

    eth_module.register_async_method(
        "eth_sendTransaction",
        async |params, ctx, _| -> RpcResult<TxHash> {
            let tx_req: TransactionRequest = params.one()?;
            match make_interactive_request(
                ctx.sender.clone(),
                InteractiveRequest::EthRequestAccounts,
            )
            .await
            .map_err(json_rpc_internal_error)?
            {
                InteractiveResponse::EthRequestAccounts(items) => {
                    if let Some(signer_addr) = items.first() {
                        let provider = (*ctx.provider).clone();
                        let provider = provider.join_with(WalletFiller::new(RpcSigner::new(
                            *signer_addr,
                            ctx.sender.clone(),
                        )));
                        let tx = provider
                            .send_transaction(tx_req)
                            .await
                            .map_err(json_rpc_internal_error)?;
                        Ok(*tx.tx_hash())
                    } else {
                        Err(ErrorObject::from(ErrorCode::InternalError))
                    }
                }
                _ => Err(ErrorObject::from(ErrorCode::InternalError)),
            }
        },
    )?;

    eth_module.register_async_method(
        "eth_signTransaction",
        async |params, ctx, _| -> RpcResult<Bytes> {
            let tx_req: TransactionRequest = params.one()?;
            match make_interactive_request(
                ctx.sender.clone(),
                InteractiveRequest::EthRequestAccounts,
            )
            .await
            .map_err(json_rpc_internal_error)?
            {
                InteractiveResponse::EthRequestAccounts(items) => {
                    if let Some(signer_addr) = items.first() {
                        let provider = (*ctx.provider).clone();
                        let provider = provider.join_with(WalletFiller::new(RpcSigner::new(
                            *signer_addr,
                            ctx.sender.clone(),
                        )));
                        let signed_encoded_tx = provider
                            .sign_transaction(tx_req)
                            .await
                            .map_err(json_rpc_internal_error)?;
                        Ok(signed_encoded_tx)
                    } else {
                        Err(ErrorObject::from(ErrorCode::InternalError))
                    }
                }
                _ => Err(ErrorObject::from(ErrorCode::InternalError)),
            }
        },
    )?;

    eth_module.register_async_method("eth_sign", async |params, ctx, _| -> RpcResult<Bytes> {
        let (signer_addr, message) = params.parse::<(Address, Bytes)>()?;
        let (sender, receiver) = oneshot::channel::<InteractiveResponse>();
        ctx.sender
            .send((InteractiveRequest::EthSign(signer_addr, message), sender))
            .await
            .map_err(json_rpc_internal_error)?;
        let res = receiver.await.map_err(json_rpc_internal_error)?;
        match res {
            InteractiveResponse::EthSign(signature) => Ok(signature
                .map(|s| s.as_bytes().into())
                .map_err(json_rpc_internal_error)?),
            _ => Err(ErrorObject::from(ErrorCode::InternalError)),
        }
    })?;

    eth_module.register_async_method(
        "eth_signTypedData_v4",
        async |params, ctx, _| -> RpcResult<Bytes> {
            let (signer_addr, typed_data) = params.parse::<(Address, TypedData)>()?;
            let (sender, receiver) = oneshot::channel::<InteractiveResponse>();
            ctx.sender
                .send((
                    InteractiveRequest::EthSignTypedData(signer_addr, typed_data.into()),
                    sender,
                ))
                .await
                .map_err(json_rpc_internal_error)?;
            let res = receiver.await.map_err(json_rpc_internal_error)?;
            match res {
                InteractiveResponse::EthSignTypedData(signature) => Ok(signature
                    .map(|s| s.as_bytes().into())
                    .map_err(json_rpc_internal_error)?),
                _ => Err(ErrorObject::from(ErrorCode::InternalError)),
            }
        },
    )?;
    Ok(eth_module)
}

#[derive(Debug, Clone)]
struct RpcSigner {
    signer_addr: Address,
    sender: mpsc::Sender<(InteractiveRequest, oneshot::Sender<InteractiveResponse>)>,
}

impl RpcSigner {
    fn new(
        signer_addr: Address,
        sender: mpsc::Sender<(InteractiveRequest, oneshot::Sender<InteractiveResponse>)>,
    ) -> Self {
        Self {
            sender,
            signer_addr,
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum RpcSignerError {
    #[error("signer address mismatch")]
    SignerAddressMismatch,
    #[error("signing response channel dropped")]
    SignatureResponseChannelDropped,
    #[error("unexpected response")]
    UnexpectedResponse,
    #[error("sending signature request failed")]
    SendingSignatureRequestFailed,
    #[error("signing error: {0:?}")]
    SigningError(Box<dyn std::error::Error + Send + Sync>),
}

impl NetworkWallet<Ethereum> for RpcSigner {
    #[doc = " Get the default signer address. This address should be used"]
    #[doc = " in [`NetworkWallet::sign_transaction_from`] when no specific signer is"]
    #[doc = " specified."]
    fn default_signer_address(&self) -> Address {
        self.signer_addr
    }

    #[doc = " Return true if the signer contains a credential for the given address."]
    fn has_signer_for(&self, address: &Address) -> bool {
        address == &self.signer_addr
    }

    #[doc = " Return an iterator of all signer addresses."]
    fn signer_addresses(&self) -> impl Iterator<Item = Address> {
        std::iter::once(self.signer_addr)
    }

    #[doc = " Asynchronously sign an unsigned transaction, with a specified"]
    #[doc = " credential."]
    #[doc(alias = "sign_tx_from")]
    async fn sign_transaction_from(
        &self,
        sender: Address,
        tx: <Ethereum as alloy::providers::Network>::UnsignedTx,
    ) -> alloy::signers::Result<<Ethereum as Network>::TxEnvelope> {
        macro_rules! alloy_err {
            ($e:expr) => {
                alloy::signers::Error::Other(Box::new($e))
            };
        }
        macro_rules! map_err {
            ($e:expr) => {
                |_| alloy_err!($e)
            };
        }

        if sender != self.signer_addr {
            return Err(alloy_err!(RpcSignerError::SignerAddressMismatch));
        }

        let (sender, receiver) = oneshot::channel();
        self.sender
            .send((
                InteractiveRequest::SignTransaction(Box::new(tx.clone())),
                sender,
            ))
            .await
            .map_err(map_err!(RpcSignerError::SendingSignatureRequestFailed))?;

        let response = receiver
            .await
            .map_err(map_err!(RpcSignerError::SignatureResponseChannelDropped))?;
        match response {
            InteractiveResponse::SignTransaction(Ok(sig)) => {
                Ok(EthereumTxEnvelope::new_unhashed(tx, sig))
            }
            InteractiveResponse::SignTransaction(Err(e)) => {
                Err(alloy_err!(RpcSignerError::SigningError(e)))
            }
            _ => Err(alloy_err!(RpcSignerError::UnexpectedResponse)),
        }
    }
}
