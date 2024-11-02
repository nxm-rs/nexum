use std::{cell::RefCell, future::Future, rc::Rc, sync::Arc};

use chrome_sys::{port, tabs::send_message_to_tab};
use ferris_primitives::{EthPayload, MessagePayload};
use gloo_timers::callback::Timeout;
use js_sys::{Function, Reflect};
use jsonrpsee::{
    core::{client::ClientT, ClientError},
    wasm_client::Client,
};
use serde::Deserialize;
use serde_json::Value;
use serde_wasm_bindgen::from_value;
use tracing::{debug, info, trace, warn};
use wasm_bindgen::{prelude::Closure, JsCast, JsValue};
use wasm_bindgen_futures::spawn_local;

use crate::{provider::ProviderType, state::BufferedRequest, Extension, EXTENSION_PORT_NAME};

// To be used with the `chrome.runtime.onConnect` event
pub async fn runtime_on_connect(extension: Arc<Extension>, js_port: JsValue) {
    // Retrieve port name, logging on error
    trace!("Received connection: {:?}", js_port);

    #[derive(Deserialize)]
    struct Port {
        pub name: String,
    }

    let port: Port = from_value(js_port.clone()).unwrap();

    let extension_clone = extension.clone();
    if port.name == EXTENSION_PORT_NAME {
        trace!("Connection is for frame_connect");
        let mut state = extension_clone.state.lock().await;
        state.settings_panel = Some(js_port.clone());
        state.update_settings_panel();

        // Add onDisconnect listener
        port_on_disconnect(extension, js_port.clone());
    }
}

// Function to set up the on_disconnect handler
fn port_on_disconnect(extension: Arc<Extension>, port: JsValue) {
    // Create a placeholder for on_disconnect, initially set to None
    let on_disconnect: Rc<RefCell<Option<Closure<dyn Fn(JsValue)>>>> = Rc::new(RefCell::new(None));

    // Clone references to port and on_disconnect to use in the closure
    let port_clone = port.clone();
    let on_disconnect_clone = Rc::clone(&on_disconnect);

    let on_disconnect_closure = Closure::wrap(Box::new(move |_: JsValue| {
        let port_inner = port_clone.clone();
        let on_disconnect_inner = on_disconnect_clone.clone();
        let extension_inner = extension.clone();

        // Spawn a task to handle disconnection asynchronously
        spawn_local(async move {
            info!("Port disconnected");

            // Access the singleton extension instance
            let mut state = extension_inner.state.lock().await;
            if state.settings_panel == Some(port_inner.clone()) {
                debug!("Resetting settings_panel state");
                state.settings_panel = None;
                state.update_settings_panel();
            }

            // Remove the on_disconnect listener using the original closure
            if let Some(ref closure) = *on_disconnect_inner.borrow() {
                if port::remove_on_disconnect_listener(
                    port_inner.clone(),
                    closure.as_ref().unchecked_ref::<Function>(),
                )
                .is_err()
                {
                    warn!(
                        "Failed to remove onDisconnect listener for port: {:?}",
                        port_inner
                    );
                } else {
                    info!("Removed onDisconnect listener for port: {:?}", port_inner);
                }
            }
        });
    }) as Box<dyn Fn(JsValue)>);

    // Populate the on_disconnect RefCell with the actual closure
    *on_disconnect.borrow_mut() = Some(on_disconnect_closure);

    // Attach the on_disconnect handler to the port
    if let Some(ref closure) = *on_disconnect.borrow() {
        if let Err(e) =
            port::add_on_disconnect_listener(port.clone(), closure.as_ref().unchecked_ref())
        {
            warn!("Failed to add onDisconnect listener: {:?}", e);
        }
    }

    // Keep the Rc alive
    let _ = Rc::clone(&on_disconnect);
}

// To be used with the `chrome.runtime.onMessage` event
pub async fn runtime_on_message(
    extension: Arc<Extension>,
    provider: ProviderType,
    message: JsValue,
    sender: JsValue,
) {
    trace!("Received message: {:?} from {:?}", message, sender);

    let payload = match MessagePayload::from_js_value(&message) {
        Ok(payload) => payload,
        Err(e) => {
            warn!("Failed to deserialize message: {:#?}", e);
            return;
        }
    };

    match payload {
        MessagePayload::JsonResponse(p) => {
            warn!("Not implemented: handle Eth payload - {:?}", p)
        }
        MessagePayload::EthEvent(p) => {
            warn!("Not implemented: handle EthEvent payload - {:?}", p)
        }
        MessagePayload::EmbeddedAction(p) => {
            warn!("Not implemented: handle EmbeddedAction payload - {:?}", p)
        }
        MessagePayload::ChainChanged(p) => {
            warn!("Not implemented: handle ChainChanged payload - {:?}", p)
        }
        MessagePayload::JsonRequest(p) => {
            // Guarantees:
            // 1. Provider is connected
            //      i. If provider is not connected, we buffer the request
            //      ii. Once the provider is connected, we send the buffered requests
            //      iii. Buffered requests are subject to the same timeout as configured
            //           in jsonrpsee
            // 2. Request timeout (handled by jsonrpsee)
            //      i. If the request times out, we send an error response
            //
            // To handle this, we need to add a buffer to the extension state, and the buffer
            // will consist of a series of closures that will be executed once the client is connected

            let fut = create_request_task(p.clone(), provider.clone(), sender.clone());
            if provider
                .read()
                .ok()
                .and_then(|guard| guard.as_ref().map(|client| client.is_connected()))
                .unwrap_or(false)
            {
                spawn_local(fut);
            } else {
                debug!("Provider is not connected, buffering request");
                // At this point, we need to:
                // 1. Generate a unique identifier for the request
                // 2. Buffer the request
                // 3. Create a timer after which we will send an error response

                let uuid = uuid::Uuid::new_v4().to_string();

                // Spawn a timer to handle the timeout
                let extension_clone = extension.clone();
                let sender_clone = sender.clone();
                let uuid_clone = uuid.clone();
                let timer = Timeout::new(60000, move || {
                    spawn_local(async move {
                        let mut state = extension_clone.state.lock().await;
                        if let Some(_) = state.buffered_requests.remove(&uuid_clone) {
                            debug!("Request timed out: {:?}", uuid_clone);
                            let eth_payload = EthPayload {
                                base: p.base,
                                method: p.method,
                                params: p.params,
                                result: None,
                                error: Some(serde_json::Value::String(
                                    "Request timed out".to_string(),
                                )),
                            };
                            let message = MessagePayload::JsonResponse(eth_payload).to_js_value();
                            if let Some(tab_id) = tab_id_from_sender(sender_clone) {
                                if let Err(e) = send_message_to_tab(tab_id, message).await {
                                    warn!("Failed to send response to tab: {:?}", e);
                                }
                            }
                        }
                    });
                });

                // Buffer the request
                let mut state = extension.state.lock().await;
                state.buffered_requests.insert(
                    uuid,
                    BufferedRequest {
                        timer,
                        future: Box::pin(fut),
                    },
                );
            }
        }
    }
}

async fn handle_request(client: &Client, p: &EthPayload) -> EthPayload {
    match client
        .request::<Value, _>(
            &p.method.clone().unwrap_or_default(),
            p.params.clone().unwrap_or_default(),
        )
        .await
    {
        Ok(response) => EthPayload {
            base: p.base.clone(),
            method: p.method.clone(),
            params: p.params.clone(),
            result: Some(response),
            error: None,
        },
        Err(error) => {
            let error_message = match &error {
                ClientError::Call(call_error) => {
                    trace!("Call error: {:?}", call_error);
                    serde_json::to_string(call_error).unwrap()
                }
                ClientError::Transport(transport_error) => {
                    trace!("Transport error: {:?}", transport_error);
                    serde_json::to_string(&transport_error.to_string()).unwrap()
                }
                _ => {
                    trace!("Other error: {:?}", error);
                    serde_json::to_string(&error.to_string()).unwrap()
                }
            };

            EthPayload {
                base: p.base.clone(),
                method: p.method.clone(),
                params: p.params.clone(),
                result: None,
                error: Some(Value::String(error_message)),
            }
        }
    }
}

// Function to send the EthPayload message to the tab
async fn send_response(sender: JsValue, eth_payload: EthPayload) {
    let message = MessagePayload::JsonResponse(eth_payload).to_js_value();
    trace!("Sending response: {:?}", message);

    if let Some(tab_id) = tab_id_from_sender(sender.clone()) {
        if let Err(e) = send_message_to_tab(tab_id, message).await {
            warn!("Failed to send response to tab {}: {:?}", tab_id, e);
        }
    }
}

fn create_request_task(
    p: EthPayload,
    provider: ProviderType,
    sender: JsValue,
) -> impl Future<Output = ()> {
    async move {
        let eth_payload = {
            let provider_guard = provider.read().expect("Failed to acquire read lock"); // Lock the provider (not async)
            if let Some(client) = provider_guard.as_ref() {
                handle_request(client, &p).await
            } else {
                EthPayload {
                    base: p.base.clone(),
                    method: p.method.clone(),
                    params: p.params.clone(),
                    result: None,
                    error: Some(Value::String("Client not available".to_string())),
                }
            }
        };

        send_response(sender, eth_payload).await;
    }
}

fn tab_id_from_sender(sender: JsValue) -> Option<u32> {
    Reflect::get(&sender, &JsValue::from_str("tab"))
        .and_then(|tab| Reflect::get(&tab, &JsValue::from_str("id")))
        .ok()
        .and_then(|id| id.as_f64().map(|id| id as u32))
}
