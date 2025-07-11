use alloy::providers::{Provider, fillers::TxFiller};
use jsonrpsee::RpcModule;

use crate::rpc::GlobalRpcContext;

pub fn init<F, P>(
    context: GlobalRpcContext<F, P>,
) -> eyre::Result<RpcModule<GlobalRpcContext<F, P>>>
where
    P: Provider + 'static,
    F: TxFiller + 'static,
{
    let wallet_module = RpcModule::new(context);
    Ok(wallet_module)
}
