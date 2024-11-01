use std::{cell::RefCell, rc::Rc, sync::Arc};

use chrome_sys::{port, tabs::send_message_to_tab};
use ferris_primitives::{EthPayload, MessagePayload};
use futures::lock::Mutex;
use js_sys::{Function, Reflect};
use jsonrpsee::{
    core::client::{ClientT, Error as JsonRpcError},
    wasm_client::Client,
};
use serde::Deserialize;
use serde_wasm_bindgen::from_value;
use tracing::{debug, info, trace, warn};
use wasm_bindgen::{prelude::Closure, JsCast, JsValue};
use wasm_bindgen_futures::spawn_local;

use crate::{Extension, EXTENSION_PORT_NAME};

// To be used with the `chrome.runtime.onConnect` event
pub async fn runtime_on_connect(extension: Arc<Mutex<Extension>>, js_port: JsValue) {
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
        let extension = extension.lock().await;
        let mut state = extension.state.lock().await;
        state.settings_panel = Some(js_port.clone());
        state.update_settings_panel();

        // Add onDisconnect listener
        port_on_disconnect(extension_clone, js_port.clone());
    }
}

// Function to set up the on_disconnect handler
fn port_on_disconnect(extension: Arc<Mutex<Extension>>, port: JsValue) {
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
            let extension_inner = extension_inner.lock().await;
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
pub async fn runtime_on_message(provider: Arc<Client>, message: JsValue, sender: JsValue) {
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
            let eth_payload = match provider
                .request::<serde_json::Value, _>(
                    &p.method.clone().unwrap_or_default(),
                    p.params.clone().unwrap_or_default(),
                )
                .await
            {
                Ok(response) => {
                    // Successful response
                    EthPayload {
                        base: p.base,
                        method: p.method,
                        params: p.params,
                        result: Some(response),
                        error: None,
                    }
                }
                Err(JsonRpcError::Call(call_error)) => {
                    // Call-specific error (JSON-RPC error object)
                    trace!("Call error: {:?}", call_error);
                    EthPayload {
                        base: p.base,
                        method: p.method,
                        params: p.params,
                        result: None,
                        error: Some(serde_json::Value::String(
                            serde_json::to_string(&call_error).unwrap(),
                        )),
                    }
                }
                Err(JsonRpcError::Transport(transport_error)) => {
                    // Transport-related error (network or HTTP issue)
                    trace!("Transport error: {:?}", transport_error);
                    EthPayload {
                        base: p.base,
                        method: p.method,
                        params: p.params,
                        result: None,
                        error: Some(serde_json::Value::String(
                            serde_json::to_string(&transport_error.to_string()).unwrap(),
                        )),
                    }
                }
                Err(err) => {
                    // Other types of errors (e.g., Parse errors, Internal errors)
                    trace!("Other error: {:?}", err);
                    EthPayload {
                        base: p.base,
                        method: p.method,
                        params: p.params,
                        result: None,
                        error: Some(serde_json::Value::String(
                            serde_json::to_string(&err.to_string()).unwrap(),
                        )),
                    }
                }
            };

            // Convert `eth_payload` to JSON to send back
            let message = MessagePayload::JsonResponse(eth_payload).to_js_value();

            trace!("Sending response: {:?}", message);

            let tab_id = Reflect::get(&sender, &JsValue::from_str("tab"))
                .and_then(|tab| Reflect::get(&tab, &JsValue::from_str("id")))
                .ok()
                .and_then(|id| id.as_f64().map(|id| id as u32));

            if let Some(tab_id) = tab_id {
                send_message_to_tab(tab_id, message).await;
            }
        }
    }
}
