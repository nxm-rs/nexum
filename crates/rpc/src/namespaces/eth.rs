use jsonrpsee::{
    core::RpcResult,
    types::{ErrorCode, ErrorObject},
    ws_client::WsClient,
    RpcModule,
};
use std::sync::Arc;
use tokio::sync::oneshot;

use crate::rpc::{
    json_rpc_internal_error, upstream_request, GlobalRpcContext, InteractiveRequest,
    InteractiveResponse,
};

pub fn init(context: GlobalRpcContext, client: Arc<WsClient>) -> RpcModule<GlobalRpcContext> {
    let mut eth_module = RpcModule::new(context);
    let eth_methods = vec![
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
    ];
    eth_methods.iter().for_each(|method| {
        let _ = eth_module.register_async_method(method, upstream_request(method, client.clone()));
    });

    let _ = eth_module.register_async_method(
        "eth_requestAccounts",
        async |_, ctx, _| -> RpcResult<Vec<String>> {
            let (sender, receiver) = oneshot::channel::<InteractiveResponse>();
            ctx.sender
                .send((InteractiveRequest::EthRequestAccounts, sender))
                .await
                .map_err(json_rpc_internal_error)?;
            let response = receiver.await.map_err(json_rpc_internal_error)?;
            match response {
                InteractiveResponse::EthRequestAccounts(accounts) => Ok(accounts),
                _ => Err(ErrorObject::from(ErrorCode::InternalError)),
            }
        },
    );

    let _ = eth_module.register_async_method(
        "eth_accounts",
        async |_, ctx, _| -> RpcResult<Vec<String>> {
            let (sender, receiver) = oneshot::channel::<InteractiveResponse>();
            ctx.sender
                .send((InteractiveRequest::EthAccounts, sender))
                .await
                .map_err(json_rpc_internal_error)?;
            let response = receiver.await.map_err(json_rpc_internal_error)?;
            match response {
                InteractiveResponse::EthAccounts(accounts) => Ok(accounts),
                _ => Err(ErrorObject::from(ErrorCode::InternalError)),
            }
        },
    );

    eth_module
}
