use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use futures::future::BoxFuture;
use futures::FutureExt;
use jsonrpsee::core::client::ClientT;
use jsonrpsee::core::traits::ToRpcParams;
use jsonrpsee::core::{ClientError, RpcResult};
use jsonrpsee::server::middleware::rpc::{RpcServiceBuilder, RpcServiceT};
use jsonrpsee::server::{
    serve_with_graceful_shutdown, stop_channel, ServerHandle, StopHandle, TowerServiceBuilder,
};
use jsonrpsee::types::{ErrorCode, ErrorObject, Params, Request};
use jsonrpsee::ws_client::{WsClient, WsClientBuilder};
use jsonrpsee::{MethodResponse, Methods, RpcModule};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::net::TcpListener;
use tower::Service;
use tracing::trace;

use crate::namespaces::{eth, net, wallet, web3};

#[derive(Clone, Debug, Default)]
struct Metrics {
    opened_ws_connections: Arc<AtomicUsize>,
    closed_ws_connections: Arc<AtomicUsize>,
    http_calls: Arc<AtomicUsize>,
    success_http_calls: Arc<AtomicUsize>,
}

/// Request parameters
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum RequestParams {
    /// no parameters provided
    None,
    /// An array of JSON values
    Array(Vec<serde_json::Value>),
    /// a map of JSON values
    Object(serde_json::Map<String, serde_json::Value>),
}

impl From<RequestParams> for serde_json::Value {
    fn from(params: RequestParams) -> Self {
        match params {
            RequestParams::None => Self::Null,
            RequestParams::Array(arr) => arr.into(),
            RequestParams::Object(obj) => obj.into(),
        }
    }
}

impl ToRpcParams for RequestParams {
    fn to_rpc_params(self) -> Result<Option<Box<serde_json::value::RawValue>>, serde_json::Error> {
        let json = serde_json::to_string(&self)?;
        serde_json::value::RawValue::from_string(json).map(Some)
    }
}

// It's possible to access the connection ID
// by using the low-level API.
#[derive(Clone)]
pub struct CallerContext<S> {
    service: S,
}

impl<'a, S> RpcServiceT<'a> for CallerContext<S>
where
    S: RpcServiceT<'a> + Send + Sync + Clone + 'static,
{
    type Future = BoxFuture<'a, MethodResponse>;

    fn call(&self, req: Request<'a>) -> Self::Future {
        let service = self.service.clone();

        async move {
            trace!("Request: {:?}", req);
            let rp = service.call(req).await;
            rp
        }
        .boxed()
    }
}

pub async fn run(addr: String, rpc_url: String) -> anyhow::Result<ServerHandle> {
    let addr = addr.parse::<SocketAddr>()?;
    run_server(addr, rpc_url.as_str()).await
}

// Define a function that returns the future
pub fn upstream_request(
    method_name: &'static str,
    shared_client: Arc<WsClient>,
) -> impl Fn(Params<'static>, Arc<()>, jsonrpsee::Extensions) -> BoxFuture<'static, RpcResult<Value>>
       + Send
       + Sync
       + Clone
       + 'static {
    let method_name: Arc<String> = Arc::new(method_name.to_string());
    let shared_client = Arc::clone(&shared_client);

    move |params: Params<'static>,
          _: Arc<()>,
          _: jsonrpsee::Extensions|
          -> BoxFuture<'static, RpcResult<Value>> {
        let client = Arc::clone(&shared_client);
        let method_name = Arc::clone(&method_name);

        let params: Result<RequestParams, _> = params.parse();
        trace!("Received request extension");
        trace!(
            "Received request: {} with params: {:?}",
            *method_name,
            params
        );

        async move {
            let params: RequestParams = match params {
                Ok(params) => params,
                Err(_) => return Err(ErrorObject::from(ErrorCode::ParseError)),
            };

            // Perform the request
            let response: Result<Value, ClientError> = client.request(&method_name, params).await;

            // Match the result and convert errors
            match response {
                Ok(res) => Ok(res),
                Err(_) => Err(ErrorObject::from(ErrorCode::InternalError)),
            }
        }
        .boxed() // Box the future to return it as BoxFuture
    }
}

pub async fn run_server(listen_addr: SocketAddr, rpc_url: &str) -> anyhow::Result<ServerHandle> {
    let listener = TcpListener::bind(listen_addr).await?;
    let rpc: Arc<WsClient> = Arc::new(WsClientBuilder::default().build(rpc_url).await?);

    // This state is cloned for every connection all these types based on Arcs and it should
    // be relatively cheap to clone them.
    //
    // Make sure that nothing expensive is cloned here when doing this or use an `Arc`.
    #[derive(Clone)]
    struct PerConnection<RpcMiddleware, HttpMiddleware> {
        methods: Methods,
        stop_handle: StopHandle,
        metrics: Metrics,
        svc_builder: TowerServiceBuilder<RpcMiddleware, HttpMiddleware>,
    }

    // Each RPC call/connection get its own `stop_handle` to able to determine whether the server
    // has been stopped or not. To keep the server running the `server_handle` must be kept and it
    // can also be used to stop the server.
    let (stop_handle, server_handle) = stop_channel();

    let mut methods = RpcModule::new(());
    methods.merge(eth::init((), rpc.clone())).unwrap();
    methods.merge(net::init((), rpc.clone())).unwrap();
    methods.merge(web3::init((), rpc.clone())).unwrap();
    methods.merge(wallet::init((), rpc.clone())).unwrap();

    let per_conn_template = PerConnection {
        methods: methods.into(),
        stop_handle: stop_handle.clone(),
        svc_builder: jsonrpsee::server::Server::builder()
            .max_connections(33)
            .to_service_builder(),
        metrics: Metrics::default(),
    };

    tokio::spawn(async move {
        loop {
            // The `tokio::select!` macro is used to wait for either of the
            // listeners to accept a new connection or for the server to be
            // stopped.
            let sock = tokio::select! {
                res = listener.accept() => {
                    match res {
                        Ok((stream, _remote_addr)) => stream,
                        Err(e) => {
                            tracing::error!("failed to accept v4 connection: {:?}", e);
                            continue;
                        }
                    }
                }
                _ = per_conn_template.stop_handle.clone().shutdown() => break,
            };
            let per_conn = per_conn_template.clone();

            let svc = tower::service_fn(move |req: hyper::Request<hyper::body::Incoming>| {
                let is_websocket = jsonrpsee::server::ws::is_upgrade_request(&req);
                let transport_label = if is_websocket { "ws" } else { "http" };
                let PerConnection {
                    methods,
                    stop_handle,
                    metrics,
                    svc_builder,
                    ..
                } = per_conn.clone();

                let rpc_middleware = RpcServiceBuilder::new()
                    .rpc_logger(1024)
                    .layer_fn(move |service| CallerContext { service });

                let mut svc = svc_builder
                    // .set_http_middleware(http_middleware)
                    .set_rpc_middleware(rpc_middleware)
                    .build(methods, stop_handle);

                if is_websocket {
                    // Utilize the session close future to know when the actual WebSocket
                    // session was closed.
                    let session_close = svc.on_session_closed();

                    // A little bit weird API but the response to HTTP request must be returned below
                    // and we spawn a task to register when the session is closed.
                    tokio::spawn(async move {
                        session_close.await;
                        tracing::info!("Closed WebSocket connection");
                        metrics
                            .closed_ws_connections
                            .fetch_add(1, Ordering::Relaxed);
                    });

                    async move {
                        tracing::info!("Opened WebSocket connection");
                        metrics
                            .opened_ws_connections
                            .fetch_add(1, Ordering::Relaxed);
                        // https://github.com/rust-lang/rust/issues/102211 the error type can't be inferred
                        // to be `Box<dyn std::error::Error + Send + Sync>` so we need to convert it to a concrete type
                        // as workaround.
                        svc.call(req).await.map_err(|e| anyhow::anyhow!("{:?}", e))
                    }
                    .boxed()
                } else {
                    // HTTP.
                    async move {
                        tracing::info!("Opened HTTP connection");
                        metrics.http_calls.fetch_add(1, Ordering::Relaxed);
                        let rp = svc.call(req).await;

                        if rp.is_ok() {
                            metrics.success_http_calls.fetch_add(1, Ordering::Relaxed);
                        }

                        tracing::info!("Closed HTTP connection");
                        // https://github.com/rust-lang/rust/issues/102211 the error type can't be inferred
                        // to be `Box<dyn std::error::Error + Send + Sync>` so we need to convert it to a concrete type
                        // as workaround.
                        rp.map_err(|e| anyhow::anyhow!("{:?}", e))
                    }
                    .boxed()
                }
            });

            tokio::spawn(serve_with_graceful_shutdown(
                sock,
                svc,
                stop_handle.clone().shutdown(),
            ));
        }
    });

    Ok(server_handle)
}
