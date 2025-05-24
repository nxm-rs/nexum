use alloy::providers::Provider;
use jsonrpsee::RpcModule;
use std::sync::Arc;

use crate::{rpc::GlobalRpcContext, upstream_requests};

pub fn init<P>(c: GlobalRpcContext<P>) -> eyre::Result<RpcModule<GlobalRpcContext<P>>>
where
    P: Provider + 'static,
{
    let mut web3_module = RpcModule::new(c);
    upstream_requests!(web3_module, "web3_clientVersion");
    Ok(web3_module)
}
