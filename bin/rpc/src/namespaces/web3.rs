use std::sync::Arc;
use jsonrpsee::{ws_client::WsClient, RpcModule};

use crate::rpc::upstream_request;

pub type Web3Context = ();

pub fn init(c: Web3Context, client: Arc<WsClient>) -> RpcModule<Web3Context> {
    let mut web3_module = RpcModule::new(c);

    let web3_methods = vec![
        "web3_clientVersion",
    ];
    web3_methods.iter().for_each(|method| {
        let _ = web3_module.register_async_method(method, upstream_request(method, client.clone()));
    });

    web3_module
}
