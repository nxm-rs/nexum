pub mod eth;
pub mod net;
pub mod wallet;
pub mod web3;

#[macro_export]
macro_rules! upstream_request {
    ($method_name:literal) => {
        pastey::paste! {
        #[allow(non_snake_case)]
        async fn [<upstream_ $method_name>]<F, P> (
            params: jsonrpsee::types::Params<'static>,
            context: Arc<$crate::rpc::GlobalRpcContext<F, P>>,
            _: jsonrpsee::Extensions,
        ) -> jsonrpsee::core::RpcResult<serde_json::Value>
        where
            P: alloy::providers::Provider,
            F: alloy::providers::fillers::TxFiller,
        {
            let params: Result<$crate::rpc::RequestParams, _> = params.parse();
            tracing::trace!("Received request extension");
            tracing::trace!(
                "Received request: {} with params: {:?}",
                $method_name,
                params
            );

            let params: $crate::rpc::RequestParams = match params {
                Ok(params) => params,
                Err(_) => return Err(jsonrpsee::types::ErrorObject::from(jsonrpsee::types::ErrorCode::ParseError)),
            };

            // Perform the request
            let response: Result<serde_json::Value, alloy::transports::RpcError<alloy::transports::TransportErrorKind>> = context.provider
                .raw_request(std::borrow::Cow::Borrowed($method_name), params)
                .await;

            // Match the result and convert errors
            match response {
                Ok(res) => Ok(res),
                Err(_) => Err(jsonrpsee::types::ErrorObject::from(jsonrpsee::types::ErrorCode::InternalError)),
            }
        }
        }
    };
}

#[macro_export]
macro_rules! upstream_requests {
    ($rpc_module:ident, $($method_name:literal),+) => {
        $(
            $crate::upstream_request!($method_name);
            pastey::paste! {
            $rpc_module.register_async_method($method_name, [<upstream_ $method_name>])?;
            }
        )+
    };
}
