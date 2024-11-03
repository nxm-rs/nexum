use std::{
    sync::{Arc, RwLock},
    time::Duration,
};

use chrome_sys::tabs;
use futures::StreamExt;
use gloo_timers::future::IntervalStream;
use jsonrpsee::wasm_client::{Client, WasmClientBuilder};
use tracing::{debug, trace, warn};
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;

use crate::{events::send_event, ConnectionState, Extension};

pub type ProviderType = Arc<RwLock<Option<Client>>>;
const UPSTREAM_URL: &str = "ws://127.0.0.1:1248";

/// Helper function to create a new JSON-RPC client
pub async fn create_provider() -> Result<Client, JsValue> {
    WasmClientBuilder::default()
        .request_timeout(Duration::from_secs(60))
        .build(UPSTREAM_URL)
        .await
        .map_err(|e| JsValue::from_str(&format!("Failed to create provider: {:?}", e)))
}

/// Helper function to access the client if connected
fn with_connected_client<F>(provider: &ProviderType, action: F)
where
    F: FnOnce(&Client),
{
    if let Ok(provider_guard) = provider.read() {
        if let Some(client) = provider_guard.as_ref() {
            if client.is_connected() {
                action(client);
            }
        }
    }
}

/// Helper function to update provider with write lock
fn with_write_lock<F>(provider: &Option<ProviderType>, action: F)
where
    F: FnOnce(&mut Option<Client>),
{
    if let Some(provider) = provider {
        if let Ok(mut provider_guard) = provider.write() {
            action(&mut *provider_guard);
        }
    }
}

/// Executes logic when the provider connects
async fn on_connect(extension: Arc<Extension>) {
    {
        let mut state = extension.state.lock().await;
        state.set_frame_connected(ConnectionState::Connected);
    }

    send_event("connect", None, tabs::Query::default()).await;
    send_buffered_requests(extension).await;
}

/// Executes logic when the provider disconnects
async fn on_disconnect(extension: Arc<Extension>) {
    {
        let mut state = extension.state.lock().await;
        state.set_frame_connected(ConnectionState::Disconnected);
    }

    send_event("disconnect", None, tabs::Query::default()).await;
}

/// Sends all buffered RPC requests
async fn send_buffered_requests(extension: Arc<Extension>) {
    let mut state = extension.state.lock().await;
    state.buffered_requests.drain().for_each(|req| {
        debug!("Flushing buffered request: {:?}", req.0);
        spawn_local(req.1.future);
    });
}

/// Initializes the provider and starts monitoring its connection status
pub(crate) async fn init_provider(extension: Arc<Extension>) {
    if let Some(provider) = &extension.provider {
        with_connected_client(provider, |_| {
            warn!("Provider already initialized and connected");
        });
    }

    match create_provider().await {
        Ok(client) => {
            with_write_lock(&extension.provider, |provider_guard| {
                *provider_guard = Some(client);
            });
            on_connect(extension.clone()).await;
        }
        Err(e) => {
            with_write_lock(&extension.provider, |provider_guard| {
                *provider_guard = None;
            });
            warn!(error = ?e, "Failed to initialize JSON-RPC client");
        }
    }
}

/// Destroys the provider connection
pub(crate) async fn destroy_provider(extension: Arc<Extension>) {
    with_write_lock(&extension.provider, |provider_guard| {
        if provider_guard.take().is_some() {
            debug!("Provider destroyed");
        }
    });
    on_disconnect(extension).await;
}

/// Monitors the provider's connection status, emitting disconnect and reconnect events as needed
pub(crate) fn monitor_provider(provider: ProviderType, extension: Arc<Extension>) {
    let disconnected = Arc::new(RwLock::new(false));
    let reconnecting = Arc::new(RwLock::new(false));

    spawn_local(async move {
        let mut interval = IntervalStream::new(5000).fuse();
        while interval.next().await.is_some() {
            let is_disconnected = *disconnected.read().unwrap();
            let is_reconnecting = *reconnecting.read().unwrap();

            with_connected_client(&provider, |_| {
                *disconnected.write().unwrap() = false;
                trace!("Monitor: Provider is connected");
            });

            // If the provider is connected, skip the rest of the loop iteration.
            if !*disconnected.read().unwrap() {
                continue;
            }

            if !is_disconnected && !is_reconnecting {
                *disconnected.write().unwrap() = true;
                debug!("Provider disconnected");
                on_disconnect(extension.clone()).await;
            }

            if is_reconnecting {
                continue;
            }

            *reconnecting.write().unwrap() = true;
            debug!("Attempting to reconnect provider...");

            match create_provider().await {
                Ok(client) => {
                    with_write_lock(&Some(provider.clone()), |provider_guard| {
                        *provider_guard = Some(client);
                    });
                    on_connect(extension.clone()).await;
                    *reconnecting.write().unwrap() = false;
                }
                Err(e) => {
                    warn!(error = ?e, "Reconnection attempt failed");
                    *reconnecting.write().unwrap() = false;
                }
            }
        }
    });
}
