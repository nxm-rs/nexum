use std::sync::Arc;
use jsonrpsee::{core::RpcResult, ws_client::WsClient, RpcModule};

use crate::rpc::upstream_request;

pub type EthContext = ();

pub fn init(_: EthContext, client: Arc<WsClient>) -> RpcModule<EthContext> {
    let mut eth_module = RpcModule::new(());
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
    
    let _ = eth_module.register_method("eth_requestAccounts", |_, _, _| -> RpcResult<Vec<String>> {
        let addresses: Vec<String> = vec!["0xE618050F1adb1F6bb7d03A3484346AC42F3E71EE".to_string()];
        Ok(addresses)
    });
    
    let _ = eth_module.register_method("eth_accounts", |_, _, _| -> RpcResult<Vec<String>> {
        let addresses: Vec<String> = vec!["0xE618050F1adb1F6bb7d03A3484346AC42F3E71EE".to_string()];
        Ok(addresses)
    });

    eth_module
}
