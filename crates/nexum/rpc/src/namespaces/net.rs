use alloy::providers::{fillers::TxFiller, Provider};
use jsonrpsee::RpcModule;
use std::sync::Arc;

use crate::{rpc::GlobalRpcContext, upstream_requests};

pub fn init<F, P>(
    context: GlobalRpcContext<F, P>,
) -> eyre::Result<RpcModule<GlobalRpcContext<F, P>>>
where
    P: Provider + 'static,
    F: TxFiller + 'static,
{
    let mut net_module = RpcModule::new(context);
    upstream_requests!(net_module, "net_version");
    Ok(net_module)
}
