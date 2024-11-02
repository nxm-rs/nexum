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

use crate::{events::send_event, Extension};

pub type ProviderType = Arc<RwLock<Option<Client>>>;

/// Creates a new JSON-RPC client
pub(crate) async fn create_provider() -> Result<Client, JsValue> {
    let provider = WasmClientBuilder::default()
        .request_timeout(Duration::from_secs(60))
        .build("ws://127.0.0.1:1248")
        .await;

    match provider {
        Ok(client) => Ok(client),
        Err(e) => Err(JsValue::from_str(&format!(
            "Failed to create provider: {:?}",
            e
        ))),
    }
}

/// Executes logic needed when the provider connects
async fn on_connect(extension: Arc<Extension>) {
    let mut state = extension.state.lock().await;
    state.set_frame_connected(true);
    debug!("Provider connected");

    // Emit the "connect" event
    drop(state); // Release lock on state before sending events
    send_event("connect", None, tabs::Query::default()).await;

    // Send buffered RPC requests
    send_buffered_requests(extension).await;
}

/// Executes logic needed when the provider disconnects
async fn on_disconnect(extension: Arc<Extension>) {
    let mut state = extension.state.lock().await;
    state.set_frame_connected(false);
    debug!("Provider disconnected");

    // Emit the "disconnect" event
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
    // Early check: Read lock on provider to check if it's already initialized and connected
    if let Some(provider) = extension.provider.as_ref() {
        if let Ok(provider_guard) = provider.read() {
            if let Some(client) = provider_guard.as_ref() {
                if client.is_connected() {
                    warn!("Provider already initialized and connected");
                    return;
                }
            }
        }
    }

    // Attempt to initialize the provider
    match create_provider().await {
        Ok(client) => {
            // Write lock to update provider after successful connection
            if let Some(provider) = extension.provider.as_ref() {
                if let Ok(mut provider_guard) = provider.write() {
                    *provider_guard = Some(client);
                }
            }

            // Execute the on-connect logic
            on_connect(extension.clone()).await;
        }
        Err(e) => {
            // If building the client fails, set provider to None
            if let Some(provider) = extension.provider.as_ref() {
                if let Ok(mut provider_guard) = provider.write() {
                    *provider_guard = None;
                }
            }
            warn!(error = ?e, "Failed to initialize JSON-RPC client");
        }
    }
}

/// Destroys the provider connection
pub(crate) async fn destroy_provider(extension: Arc<Extension>) {
    // Write lock to remove the provider if it exists
    if let Some(provider) = extension.provider.as_ref() {
        if let Ok(mut provider_guard) = provider.write() {
            if provider_guard.take().is_some() {
                debug!("Provider destroyed");
            }
        }
    }

    // Execute the on-disconnect logic
    on_disconnect(extension).await;
}

/// Monitors the provider's connection status, emitting disconnect and reconnect events as needed
pub(crate) fn monitor_provider(provider: ProviderType, extension: Arc<Extension>) {
    // Tracking whether the provider is disconnected and waiting for reconnection
    let disconnected = Arc::new(RwLock::new(false));
    let reconnecting = Arc::new(RwLock::new(false));

    let monitor_interval = IntervalStream::new(5000);
    let disconnected_clone = disconnected.clone();
    let reconnecting_clone = reconnecting.clone();
    let provider_clone = provider.clone();
    let extension_clone = extension.clone();

    spawn_local(async move {
        let mut interval = monitor_interval.fuse();
        while let Some(_) = interval.next().await {
            let is_disconnected = *disconnected.read().unwrap();
            let is_reconnecting = *reconnecting.read().unwrap();

            if let Ok(provider_guard) = provider.read() {
                if let Some(client) = provider_guard.as_ref() {
                    if client.is_connected() {
                        // Reset disconnected flag if connected
                        if is_disconnected {
                            *disconnected.write().unwrap() = false;
                        }
                        trace!("Monitor: Provider is connected");
                        continue;
                    }
                }
            }

            // If disconnected and not reconnecting, emit disconnect event and start reconnection
            if !is_disconnected && !is_reconnecting {
                *disconnected_clone.write().unwrap() = true;
                debug!("Provider disconnected");

                // Execute the on-disconnect logic
                on_disconnect(extension_clone.clone()).await;
            }

            // If already reconnecting, skip further reconnection attempts
            if is_reconnecting {
                continue;
            }

            // Set reconnecting flag and attempt to reconnect
            *reconnecting_clone.write().unwrap() = true;
            debug!("Attempting to reconnect provider...");

            match create_provider().await {
                Ok(client) => {
                    if let Ok(mut provider_guard) = provider_clone.write() {
                        *provider_guard = Some(client);
                    }

                    // Execute the on-connect logic
                    on_connect(extension_clone.clone()).await;

                    // Reset reconnecting flag
                    *reconnecting_clone.write().unwrap() = false;
                }
                Err(e) => {
                    warn!(error = ?e, "Reconnection attempt failed");
                    *reconnecting_clone.write().unwrap() = false; // Reset reconnecting on failure
                }
            }
        }
    });
}
