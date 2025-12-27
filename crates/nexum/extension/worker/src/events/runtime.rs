use std::{future::Future, str::FromStr, sync::Arc};

use alloy_chains::Chain;
use gloo_timers::callback::Timeout;
use js_sys::Reflect;
use nexum_chrome_gloo::events::EventListener1;
use nexum_chrome_gloo::tabs::{self, Tab};
use nexum_chrome_sys::runtime::Port;
use nexum_primitives::{Error, MessageType, ProtocolMessage, RequestWithId, ResponseWithId};
use serde_json::Value;
use tracing::{debug, info, trace, warn};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

use crate::{EXTENSION_PORT_NAME, Extension, provider::Provider, state::BufferedRequest};

// To be used with the `chrome.runtime.onConnect` event
pub async fn runtime_on_connect(extension: Arc<Extension>, js_port: JsValue) {
    trace!("Received connection: {:?}", js_port);

    // Cast to Port type
    let port: Port = js_port.unchecked_into();

    if port.get_name() == EXTENSION_PORT_NAME {
        trace!("Connection is for frame_connect");

        // Set up onDisconnect listener before storing
        let extension_clone = extension.clone();
        let listener = EventListener1::new(&port.on_disconnect(), move |_: JsValue| {
            let ext = extension_clone.clone();
            spawn_local(async move {
                info!("Port disconnected");
                let mut state = ext.state.lock().await;
                state.settings_panel = None;
                state.update_settings_panel();
            });
        });

        match listener {
            Ok(l) => l.forget(), // Keep listener alive
            Err(e) => warn!("Failed to add onDisconnect listener: {:?}", e),
        }

        let mut state = extension.state.lock().await;
        state.settings_panel = Some(port);
        state.update_settings_panel();
    }
}

// Handles messages received through `chrome.runtime.onMessage` event
pub async fn runtime_on_message(extension: Arc<Extension>, message: JsValue, sender: JsValue) {
    trace!("Received message: {:?} from {:?}", message, sender);

    match ProtocolMessage::from_js_value(&message) {
        Ok(protocol_message) => {
            if let MessageType::Request(request) = protocol_message.message {
                handle_request(extension.clone(), request, sender).await;
            }
        }
        Err(_) => trace!("Payload is not a ProtocolMessage."),
    }
}

// Handles the parsed request by determining its method and handling accordingly
async fn handle_request(extension: Arc<Extension>, request: RequestWithId, sender: JsValue) {
    trace!("Received request: {:?}", request);

    match request.request.method.as_str() {
        "frame_summon" => {
            unimplemented!("Handle frame_summon request - {:?}", request.request);
        }
        "embedded_action_res" => handle_embedded_action(extension, &request.request.params).await,
        _ => handle_provider_request(extension, request, sender).await,
    }
}

// Processes `embedded_action_res` requests and sets the chain if applicable
async fn handle_embedded_action(extension: Arc<Extension>, params: &Option<Vec<Value>>) {
    if let Some(params) = params {
        if let (Some(action), Some(res)) = (params.first(), params.get(1)) {
            if action.get("type") == Some(&Value::String("getChainId".to_string())) {
                if let Some(chain_id_str) = res.get("chainId").and_then(Value::as_str) {
                    match Chain::from_str(chain_id_str) {
                        Ok(chain) => {
                            extension.state.lock().await.set_current_chain(chain);
                            return;
                        }
                        Err(e) => {
                            warn!("Unable to parse chain: {:?}", e);
                        }
                    }
                }
            }
        }
    }
    warn!("Failed to handle embedded_action_res: {:?}", params);
}

// Manages other provider requests by either forwarding or buffering them
async fn handle_provider_request(
    extension: Arc<Extension>,
    request: RequestWithId,
    sender: JsValue,
) {
    let provider = match extension.provider.as_ref() {
        Some(provider) => provider.clone(),
        None => {
            warn!("Provider not initialized.");
            return;
        }
    };

    let request_task = create_request_task(request.clone(), provider.clone(), sender.clone());

    if provider.is_connected().await {
        spawn_local(request_task);
    } else {
        debug!("Provider is not connected, buffering request");

        buffer_request(extension.clone(), request_task, request.id, sender.clone()).await;
    }
}

// Buffers the request if the provider is not connected, setting a timeout to avoid indefinite waits
async fn buffer_request(
    extension: Arc<Extension>,
    request_task: impl Future<Output = ()> + 'static,
    uuid: String,
    sender: JsValue,
) {
    let extension_clone = extension.clone();
    let uuid_clone = uuid.clone();
    let timeout = Timeout::new(60000, move || {
        spawn_local(async move {
            let mut state = extension_clone.state.lock().await;
            if state.buffered_requests.remove(&uuid_clone).is_some() {
                debug!("Request timed out: {:?}", &uuid_clone);
                send_timeout_response(uuid_clone, sender).await;
            }
        });
    });

    extension.state.lock().await.buffered_requests.insert(
        uuid.to_string(),
        BufferedRequest {
            timer: timeout,
            future: Box::pin(request_task),
        },
    );
}

// Creates a task to handle provider requests by sending them to the provider
fn create_request_task(
    req: RequestWithId,
    provider: Arc<Provider>,
    sender: JsValue,
) -> impl Future<Output = ()> {
    async move {
        warn!("TODO!: Origin upstreaming not implemented");

        // Convert `Option<Vec<JsonValue>>` into a slice for `ToRpcParams` compatibility.
        let params: &[Value] = match &req.request.params {
            Some(params) => params.as_slice(),
            None => &[],
        };

        let result = match provider.request::<Value>(&req.request.method, params).await {
            Ok(res) => Ok(res),
            Err(err) => {
                tracing::error!(?err, ?params, "rpc error");
                Err(Error {
                    code: -1,
                    message: "Client not available".to_string(),
                    data: None,
                })
            }
        };

        send_response(
            sender,
            ProtocolMessage::new(MessageType::Response(ResponseWithId { id: req.id, result })),
        )
        .await;
    }
}

// Sends a timeout response if the request could not be processed within the buffer time
async fn send_timeout_response(uuid: String, sender: JsValue) {
    let error_message = ProtocolMessage::new(MessageType::Response(ResponseWithId {
        id: uuid,
        result: Err(Error {
            code: -1,
            message: "Request timed out".to_string(),
            data: None,
        }),
    }));
    send_response(sender, error_message).await;
}

// Sends the response to the specified sender's tab
async fn send_response(sender: JsValue, message: ProtocolMessage) {
    trace!("Sending response: {:?}", message);
    if let Some(tab_id) = tab_id_from_sender(&sender) {
        if let Err(e) = tabs::send_message(tab_id, JsValue::from(message)).await {
            warn!(tab_id, error = ?e, "Failed to send response to tab");
        }
    }
}

fn tab_id_from_sender(sender: &JsValue) -> Option<i32> {
    // Retrieve the `tab` field from `sender` and get its id
    Reflect::get(sender, &JsValue::from_str("tab"))
        .ok()
        .and_then(|tab_js| {
            let tab: Tab = tab_js.unchecked_into();
            tab.get_id()
        })
}
