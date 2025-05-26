use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use alloy::consensus::{EthereumTypedTransaction, TxEip4844Variant};
use alloy::primitives::Address;
use alloy::providers::fillers::{
    BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller, TxFiller,
};
use alloy::providers::{Provider, ProviderBuilder, RootProvider};
use alloy::signers::Signature;
use alloy_chains::NamedChain;
use eyre::OptionExt;
use futures::future::BoxFuture;
use futures::FutureExt;
use jsonrpsee::core::traits::ToRpcParams;
use jsonrpsee::server::middleware::rpc::{RpcServiceBuilder, RpcServiceT};
use jsonrpsee::server::{
    serve_with_graceful_shutdown, stop_channel, ServerHandle, StopHandle, TowerServiceBuilder,
};
use jsonrpsee::types::{ErrorCode, ErrorObject, ErrorObjectOwned, Request};
use jsonrpsee::{MethodResponse, RpcModule};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tokio::sync::mpsc::{self, Receiver};
use tokio::sync::oneshot;
use tower::Service;
use tracing::trace;
use url::Url;

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

/// Requests that need some interactive or external input to compute the response
pub enum InteractiveRequest {
    EthRequestAccounts,
    EthAccounts,
    SignTransaction(Box<EthereumTypedTransaction<TxEip4844Variant>>),
}

/// Responses for the interactive requests
#[derive(Debug)]
pub enum InteractiveResponse {
    EthRequestAccounts(Vec<Address>),
    EthAccounts(Vec<Address>),
    SignTransaction(Result<Signature, Box<dyn std::error::Error + Send + Sync>>),
}

pub async fn make_interactive_request(
    sender: mpsc::Sender<(InteractiveRequest, oneshot::Sender<InteractiveResponse>)>,
    request: InteractiveRequest,
) -> eyre::Result<InteractiveResponse> {
    let (res_sender, receiver) = oneshot::channel::<InteractiveResponse>();
    sender.send((request, res_sender)).await?;
    Ok(receiver.await?)
}

#[derive(Clone, Debug)]
pub struct GlobalRpcContext<F: TxFiller, P: Provider> {
    pub sender: mpsc::Sender<(InteractiveRequest, oneshot::Sender<InteractiveResponse>)>,
    pub provider: Arc<FillProvider<F, P>>,
}

pub fn json_rpc_internal_error<E>(err: E) -> ErrorObjectOwned
where
    E: std::fmt::Debug,
{
    ErrorObject::owned(
        ErrorCode::InternalError.code(),
        format!("{err:?}"),
        None::<()>,
    )
}

pub struct RpcServerBuilder {
    rpcs: HashMap<NamedChain, Url>,
    port: u16,
    host: Ipv4Addr,
}

impl Default for RpcServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl RpcServerBuilder {
    pub fn new() -> Self {
        Self {
            rpcs: HashMap::new(),
            port: 1248,
            host: Ipv4Addr::LOCALHOST,
        }
    }

    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub fn host(mut self, host: Ipv4Addr) -> Self {
        self.host = host;
        self
    }

    pub fn chain(mut self, chain: NamedChain, rpc: Url) -> Self {
        self.rpcs.insert(chain, rpc);
        self
    }

    pub async fn build(self) -> RpcServer {
        RpcServer::new(self.rpcs, self.port, self.host).await
    }
}

pub fn chain_id_or_name_to_named_chain(chain: &str) -> eyre::Result<NamedChain> {
    let chain = chain.parse::<NamedChain>().map(Some).unwrap_or_else(|_| {
        chain
            .parse::<u64>()
            .map(|chainid| NamedChain::try_from(chainid).ok())
            .ok()
            .flatten()
    });
    chain.ok_or_eyre("failed to parse chain")
}

pub type ProviderFillers = JoinFill<
    alloy::providers::Identity,
    JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
>;
pub type ProviderWithFillers = FillProvider<ProviderFillers, RootProvider>;
pub type GlobalRpcContextT = GlobalRpcContext<ProviderFillers, RootProvider>;

pub struct RpcServer {
    rpc_urls: HashMap<NamedChain, Url>,
    providers: HashMap<NamedChain, ProviderWithFillers>,
    port: u16,
    host: Ipv4Addr,
    req_receiver:
        Option<mpsc::Receiver<(InteractiveRequest, oneshot::Sender<InteractiveResponse>)>>,
    req_sender: mpsc::Sender<(InteractiveRequest, oneshot::Sender<InteractiveResponse>)>,
    chain_methods_map: Arc<HashMap<NamedChain, RpcModule<GlobalRpcContextT>>>,
}

impl RpcServer {
    pub async fn new(rpcs: HashMap<NamedChain, Url>, port: u16, host: Ipv4Addr) -> Self {
        let (req_sender, req_receiver) = mpsc::channel(100);

        let mut this = Self {
            rpc_urls: rpcs,
            providers: Default::default(),
            port,
            host,
            req_receiver: Some(req_receiver),
            req_sender: req_sender.clone(),
            chain_methods_map: Default::default(),
        };
        this.reinit().await;
        this
    }

    pub async fn reinit(&mut self) {
        let providers = futures::future::join_all(self.rpc_urls.iter().map(|(k, v)| async {
            RootProvider::connect(v.to_string().as_str())
                .await
                .map(|p| {
                    let provider = ProviderBuilder::new().connect_provider(p);
                    (*k, provider)
                })
        }))
        .await
        .into_iter()
        .filter_map(|v| {
            v.inspect_err(|err| tracing::warn!(?err, "error establishing connection with the rpc"))
                .ok()
        })
        .collect::<HashMap<_, _>>();

        let chain_methods_map = providers
            .iter()
            .map(
                |(chain, provider)| -> eyre::Result<(NamedChain, RpcModule<GlobalRpcContextT>)> {
                    let global_ctx = GlobalRpcContext {
                        sender: self.req_sender.clone(),
                        provider: Arc::new(provider.clone()),
                    };
                    let mut methods = RpcModule::new(global_ctx.clone());
                    methods.merge(eth::init(global_ctx.clone())?)?;
                    methods.merge(net::init(global_ctx.clone())?)?;
                    methods.merge(web3::init(global_ctx.clone())?)?;
                    methods.merge(wallet::init(global_ctx.clone())?)?;
                    Ok((*chain, methods))
                },
            )
            .filter_map(|v| {
                v.inspect_err(|err| tracing::warn!(?err, "error initializing chain rpc module"))
                    .ok()
            })
            .collect::<HashMap<_, _>>();
        self.providers = providers;
        self.chain_methods_map = Arc::new(chain_methods_map);
    }

    pub async fn run(
        &mut self,
    ) -> eyre::Result<(
        ServerHandle,
        Receiver<(InteractiveRequest, oneshot::Sender<InteractiveResponse>)>,
    )> {
        let listen_addr = SocketAddr::new(self.host.into(), self.port);

        let listener = TcpListener::bind(listen_addr).await?;

        // Each RPC call/connection get its own `stop_handle` to able to determine whether the server
        // has been stopped or not. To keep the server running the `server_handle` must be kept and it
        // can also be used to stop the server.
        let (stop_handle, server_handle) = stop_channel();

        #[derive(Clone)]
        struct PerConnection<RpcMiddleware, HttpMiddleware> {
            methods: Arc<HashMap<NamedChain, RpcModule<GlobalRpcContextT>>>,
            stop_handle: StopHandle,
            metrics: Metrics,
            svc_builder: TowerServiceBuilder<RpcMiddleware, HttpMiddleware>,
        }

        let per_conn_template = PerConnection {
            methods: self.chain_methods_map.clone(),
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
                    // determine the chain of RPC
                    let chain = chain_id_or_name_to_named_chain(
                        req.uri()
                            .path()
                            .strip_prefix("/")
                            .unwrap_or_else(|| req.uri().path()),
                    );
                    if chain.is_err() {
                        return async { Err(eyre::eyre!("{:?}", chain.unwrap_err())) }.boxed();
                    }
                    let chain = chain.unwrap();

                    let PerConnection {
                        methods: chain_methods,
                        stop_handle,
                        metrics,
                        svc_builder,
                    } = per_conn.clone();
                    let methods = chain_methods.get(&chain);
                    if methods.is_none() {
                        return async { Err(eyre::eyre!("chain not configured")) }.boxed();
                    }

                    let methods = methods.unwrap().clone();

                    let is_websocket = jsonrpsee::server::ws::is_upgrade_request(&req);

                    let rpc_middleware = RpcServiceBuilder::new()
                        .rpc_logger(1024)
                        .layer_fn(move |service| CallerContext { service });

                    let mut svc = svc_builder
                        .set_rpc_middleware(rpc_middleware)
                        .build(methods, stop_handle.clone());

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

                        tracing::info!("Opened WebSocket connection");
                        metrics
                            .opened_ws_connections
                            .fetch_add(1, Ordering::Relaxed);
                        // https://github.com/rust-lang/rust/issues/102211 the error type can't be inferred
                        // to be `Box<dyn std::error::Error + Send + Sync>` so we need to convert it to a concrete type
                        // as workaround.
                        async move { svc.call(req).await.map_err(|e| eyre::eyre!("{:?}", e)) }
                            .boxed()
                    } else {
                        // HTTP.
                        tracing::info!("Opened HTTP connection");
                        metrics.http_calls.fetch_add(1, Ordering::Relaxed);
                        async move {
                            let rp = svc.call(req).await;

                            if rp.is_ok() {
                                metrics.success_http_calls.fetch_add(1, Ordering::Relaxed);
                            }

                            tracing::info!("Closed HTTP connection");
                            // https://github.com/rust-lang/rust/issues/102211 the error type can't be inferred
                            // to be `Box<dyn std::error::Error + Send + Sync>` so we need to convert it to a concrete type
                            // as workaround.
                            rp.map_err(|e| eyre::eyre!("{:?}", e))
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

        Ok((
            server_handle,
            self.req_receiver
                .take()
                .ok_or_eyre("server already running")?,
        ))
    }
}
