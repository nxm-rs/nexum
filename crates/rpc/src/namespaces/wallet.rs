use alloy::providers::Provider;
use jsonrpsee::RpcModule;

use crate::rpc::GlobalRpcContext;

pub fn init<P>(context: GlobalRpcContext<P>) -> eyre::Result<RpcModule<GlobalRpcContext<P>>>
where
    P: Provider + 'static,
{
    let wallet_module = RpcModule::new(context);
    Ok(wallet_module)
}
