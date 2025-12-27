use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

use async_lock::RwLock;
use futures::{
    StreamExt,
    future::{Either, select},
};
use nexum_chrome_gloo::tabs::QueryQueryInfo;
use gloo_timers::future::{IntervalStream, TimeoutFuture};
use jsonrpsee::{
    core::{client::ClientT, traits::ToRpcParams},
    wasm_client::{Client, WasmClientBuilder},
};
use tracing::{debug, trace, warn};
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;

use crate::{ConnectionState, Extension, events::send_event};

const UPSTREAM_URL: &str = "ws://127.0.0.1:1250/sepolia";

pub struct Provider {
    client: RwLock<Option<Client>>,
    disconnected: AtomicBool,
    reconnecting: AtomicBool,
    extension: Arc<Extension>,
}

impl Provider {
    pub fn new(extension: Arc<Extension>) -> Arc<Self> {
        Arc::new(Self {
            client: RwLock::new(None),
            disconnected: AtomicBool::new(true),
            reconnecting: AtomicBool::new(false),
            extension,
        })
    }

    /// Initialize the provider and start monitoring its connection
    pub async fn init(self: &Arc<Self>) {
        match create_client().await {
            Ok(client) => {
                self.set_client(Some(client)).await;
                if self.disconnected.swap(false, Ordering::Relaxed)
                    && self.verify_connection().await
                {
                    self.clone().on_connect().await;
                } else {
                    self.clear_client().await;
                    // Ensure `disconnected` is set to `true` after clearing client,
                    // but do not emit "disconnect" on initial failure if already disconnected.
                    if !self.disconnected.load(Ordering::Relaxed) {
                        self.disconnected.store(true, Ordering::Relaxed);
                    }
                }
            }
            Err(e) => {
                self.clear_client().await;
                self.disconnected.store(true, Ordering::Relaxed);
                warn!(error = ?e, "Failed to initialize JSON-RPC client");
            }
        }
        self.start_monitoring();
    }

    /// Reset the provider state, used when coming out of idle
    pub async fn reset(self: &Arc<Self>) {
        debug!("Resetting provider state");
        self.clone().destroy().await;
        self.init().await;
    }

    /// Destroy the provider, disconnecting the client
    async fn destroy(self: Arc<Self>) {
        self.clear_client().await;
        self.on_disconnect().await;
    }

    /// Set the client instance
    async fn set_client(&self, client: Option<Client>) {
        let mut guard = self.client.write().await;
        *guard = client;
    }

    /// Clear the client instance
    async fn clear_client(&self) {
        let mut guard = self.client.write().await;
        *guard = None;
    }

    /// Check if the provider client is currently connected
    pub async fn is_connected(&self) -> bool {
        let guard = self.client.read().await;
        if let Some(client) = &*guard {
            client.is_connected()
        } else {
            false
        }
    }

    /// Executes logic when the provider connects
    async fn on_connect(self: Arc<Self>) {
        {
            let mut state = self.extension.state.lock().await;
            state.set_frame_connected(ConnectionState::Connected);
        }

        send_event("connect", None, &QueryQueryInfo::new()).await;
        self.send_buffered_requests().await;
    }

    /// Executes logic when the provider disconnects
    async fn on_disconnect(self: Arc<Self>) {
        {
            let mut state = self.extension.state.lock().await;
            state.set_frame_connected(ConnectionState::Disconnected);
        }

        send_event("disconnect", None, &QueryQueryInfo::new()).await;
    }

    /// Helper function to verify the connection by making a lightweight RPC call
    async fn verify_connection(&self) -> bool {
        let guard = self.client.read().await;
        if let Some(client) = &*guard {
            client
                .request::<String, _>("eth_chainId", &[] as &[String])
                .await
                .is_ok()
        } else {
            false
        }
    }

    /// Sends all buffered RPC requests
    async fn send_buffered_requests(&self) {
        let mut state = self.extension.state.lock().await;
        state.buffered_requests.drain().for_each(|req| {
            debug!("Flushing buffered request: {:?}", req.0);
            spawn_local(req.1.future);
        });
    }

    /// Starts monitoring the provider connection status
    fn start_monitoring(self: &Arc<Self>) {
        let provider_clone = self.clone();
        spawn_local(async move {
            let mut interval = IntervalStream::new(5000).fuse();
            while interval.next().await.is_some() {
                if !Provider::check_and_reconnect(provider_clone.clone()).await {
                    break;
                }
            }
        });
    }

    /// Checks connection status and attempts reconnection if disconnected
    async fn check_and_reconnect(provider: Arc<Self>) -> bool {
        trace!("Monitor: checking provider connection status");

        if !provider.disconnected.load(Ordering::Relaxed) && provider.is_connected().await {
            if provider.disconnected.swap(false, Ordering::Relaxed) {
                trace!("Monitor: provider reconnected, running on_connect");

                let provider_clone = provider.clone();
                spawn_local(async move {
                    provider_clone.on_connect().await;
                });
            }
            return true;
        }

        provider.clone().handle_disconnection().await;
        provider.attempt_reconnect().await
    }

    /// Handles provider disconnection without consuming self
    async fn handle_disconnection(self: Arc<Self>) {
        if !self.disconnected.swap(true, Ordering::Relaxed) {
            trace!("Monitor: provider disconnected, running on_disconnect");

            let provider_clone = self.clone();
            spawn_local(async move {
                provider_clone.on_disconnect().await;
            });
        }
    }

    /// Attempts to reconnect the provider without consuming self
    async fn attempt_reconnect(self: Arc<Self>) -> bool {
        if !self.disconnected.load(Ordering::Relaxed) || self.reconnecting.load(Ordering::Relaxed) {
            return true;
        }

        self.reconnecting.store(true, Ordering::Relaxed);
        trace!("Monitor: attempting to reconnect provider...");

        let provider_clone = self.clone();
        spawn_local(async move {
            let timeout = TimeoutFuture::new(30_000);
            let reconnect_attempt = Box::pin(create_client());

            match select(timeout, reconnect_attempt).await {
                Either::Left(_) => {
                    trace!("Monitor: reconnection attempt timed out");
                    provider_clone.reconnecting.store(false, Ordering::Relaxed);
                }
                Either::Right((result, _)) => match result {
                    Ok(client) => {
                        provider_clone.set_client(Some(client)).await;
                        if provider_clone.verify_connection().await {
                            trace!("Monitor: reconnection attempt successful");
                            provider_clone.clone().finalize_reconnection();
                        }
                        provider_clone.reconnecting.store(false, Ordering::Relaxed);
                    }
                    Err(e) => {
                        trace!(?e, "Monitor: reconnection attempt failed");
                        provider_clone.reconnecting.store(false, Ordering::Relaxed);
                    }
                },
            }
        });

        true
    }

    /// Finalizes the reconnection process by setting connection state and triggering on_connect without consuming self
    fn finalize_reconnection(self: Arc<Self>) {
        if !self.disconnected.load(Ordering::Relaxed) {
            return;
        }
        self.disconnected.store(false, Ordering::Relaxed);
        let provider_clone = self.clone();
        spawn_local(async move {
            provider_clone.on_connect().await;
        });
    }

    /// Generalized request function to send an RPC request through the client
    pub async fn request<T: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        params: impl ToRpcParams + Send,
    ) -> Result<T, JsValue> {
        trace!("Provider::request: method={}", method);
        // Acquire a read lock on the client
        let client_guard = self.client.read().await;

        // Check if the client is available and connected
        if let Some(client) = client_guard.as_ref() {
            if client.is_connected() {
                // Perform the request and handle any errors
                client
                    .request::<T, _>(method, params)
                    .await
                    .map_err(|e| JsValue::from_str(&format!("Request failed: {e:?}")))
            } else {
                Err(JsValue::from_str("Client is not connected"))
            }
        } else {
            Err(JsValue::from_str("Client is not available"))
        }
    }
}

/// Creates a new JSON-RPC client with a timeout of 60 seconds
async fn create_client() -> Result<Client, JsValue> {
    WasmClientBuilder::default()
        .request_timeout(Duration::from_secs(60))
        .build(UPSTREAM_URL)
        .await
        .map_err(|e| JsValue::from_str(&format!("Failed to create provider: {e:?}")))
}
