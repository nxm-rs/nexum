use jsonrpsee::{ws_client::WsClient, RpcModule};
use std::sync::Arc;

use crate::rpc::upstream_request;

pub type WalletContext = ();

pub fn init(_: WalletContext, client: Arc<WsClient>) -> RpcModule<WalletContext> {
    let mut wallet_module = RpcModule::new(());
    let wallet_methods: Vec<&str> = vec![];
    wallet_methods.iter().for_each(|method| {
        let _ =
            wallet_module.register_async_method(method, upstream_request(method, client.clone()));
    });

    wallet_module
}
