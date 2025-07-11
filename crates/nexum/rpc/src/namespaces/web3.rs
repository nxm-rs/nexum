use alloy::providers::{Provider, fillers::TxFiller};
use jsonrpsee::RpcModule;
use std::sync::Arc;

use crate::{rpc::GlobalRpcContext, upstream_requests};

pub fn init<F, P>(c: GlobalRpcContext<F, P>) -> eyre::Result<RpcModule<GlobalRpcContext<F, P>>>
where
    P: Provider + 'static,
    F: TxFiller + 'static,
{
    let mut web3_module = RpcModule::new(c);
    upstream_requests!(web3_module, "web3_clientVersion");
    Ok(web3_module)
}
