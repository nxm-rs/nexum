use alloy::{
    consensus::EthereumTxEnvelope,
    network::{Ethereum, Network, NetworkWallet},
    primitives::{Address, Bytes, TxHash},
    providers::{
        fillers::{TxFiller, WalletFiller},
        Provider,
    },
    rpc::types::TransactionRequest,
};
use jsonrpsee::{
    core::RpcResult,
    types::{ErrorCode, ErrorObject},
    RpcModule,
};
use std::{error::Error, sync::Arc};
use tokio::sync::{mpsc, oneshot};

use crate::{
    rpc::{
        json_rpc_internal_error, make_interactive_request, GlobalRpcContext, InteractiveRequest,
        InteractiveResponse,
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

#[derive(Debug)]
struct SimpleError {
    msg: String,
}
impl std::fmt::Display for SimpleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SimpleError {{ {} }}", self.msg)
    }
}
impl Error for SimpleError {}

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
                alloy::signers::Error::Other(Box::new(SimpleError {
                    msg: $e.to_string(),
                }))
            };
        }
        macro_rules! map_err {
            ($e:expr) => {
                |_| alloy_err!($e)
            };
        }

        if sender != self.signer_addr {
            return Err(alloy_err!("signer address mismatch"));
        }

        let (sender, receiver) = oneshot::channel();
        self.sender
            .send((
                InteractiveRequest::SignTransaction(Box::new(tx.clone())),
                sender,
            ))
            .await
            .map_err(map_err!("sending signature request failed"))?;

        let response = receiver.await.map_err(map_err!("signing failed"))?;
        match response {
            InteractiveResponse::SignTransaction(Ok(sig)) => {
                Ok(EthereumTxEnvelope::new_unhashed(tx, sig))
            }
            InteractiveResponse::SignTransaction(Err(e)) => Err(alloy_err!(e)),
            _ => Err(alloy_err!("unexpected response")),
        }
    }
}
