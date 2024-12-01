use std::sync::Arc;
use jsonrpsee::{ws_client::WsClient, RpcModule};

use crate::rpc::upstream_request;

pub type NetContext = ();

pub fn init(_: NetContext, client: Arc<WsClient>) -> RpcModule<NetContext> {
    let mut net_module = RpcModule::new(());
    let net_methods = vec![
        "net_version",
    ];
    net_methods.iter().for_each(|method| {
        let _ = net_module.register_async_method(method, upstream_request(method, client.clone()));
    });

    net_module
}