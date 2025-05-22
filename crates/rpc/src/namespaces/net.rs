use alloy::providers::Provider;
use jsonrpsee::RpcModule;
use std::sync::Arc;

use crate::{rpc::GlobalRpcContext, upstream_requests};

pub fn init<P>(context: GlobalRpcContext<P>) -> eyre::Result<RpcModule<GlobalRpcContext<P>>>
where
    P: Provider + 'static,
{
    let mut net_module = RpcModule::new(context);
    upstream_requests!(net_module, "net_version");
    Ok(net_module)
}
