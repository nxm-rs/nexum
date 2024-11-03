use std::{cell::RefCell, future::Future, rc::Rc, sync::Arc};

use chrome_sys::{port, tabs::send_message_to_tab};
use gloo_timers::callback::Timeout;
use js_sys::{Function, Reflect};
use nexum_primitives::{EthPayload, MessagePayload};
use serde::Deserialize;
use serde_json::Value;
use serde_wasm_bindgen::from_value;
use tracing::{debug, info, trace, warn};
use wasm_bindgen::{prelude::Closure, JsCast, JsValue};
use wasm_bindgen_futures::spawn_local;

use crate::{provider::Provider, state::BufferedRequest, Extension, EXTENSION_PORT_NAME};

// To be used with the `chrome.runtime.onConnect` event
pub async fn runtime_on_connect(extension: Arc<Extension>, js_port: JsValue) {
    trace!("Received connection: {:?}", js_port);

    #[derive(Deserialize)]
    struct Port {
        pub name: String,
    }

    let port: Port = from_value(js_port.clone()).unwrap();

    if port.name == EXTENSION_PORT_NAME {
        trace!("Connection is for frame_connect");
        let mut state = extension.state.lock().await;
        state.settings_panel = Some(js_port.clone());
        state.update_settings_panel();

        // Add onDisconnect listener
        port_on_disconnect(extension.clone(), js_port.clone());
    }
}

// Function to set up the on_disconnect handler
fn port_on_disconnect(extension: Arc<Extension>, port: JsValue) {
    let on_disconnect: Rc<RefCell<Option<Closure<dyn Fn(JsValue)>>>> = Rc::new(RefCell::new(None));

    let port_clone = port.clone();
    let on_disconnect_clone = Rc::clone(&on_disconnect);

    let on_disconnect_closure = Closure::wrap(Box::new(move |_: JsValue| {
        let port_inner = port_clone.clone();
        let on_disconnect_inner = on_disconnect_clone.clone();
        let extension_inner = extension.clone();

        spawn_local(async move {
            info!("Port disconnected");

            let mut state = extension_inner.state.lock().await;
            if state.settings_panel == Some(port_inner.clone()) {
                debug!("Resetting settings_panel state");
                state.settings_panel = None;
                state.update_settings_panel();
            }

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

    *on_disconnect.borrow_mut() = Some(on_disconnect_closure);

    if let Some(ref closure) = *on_disconnect.borrow() {
        if let Err(e) =
            port::add_on_disconnect_listener(port.clone(), closure.as_ref().unchecked_ref())
        {
            warn!("Failed to add onDisconnect listener: {:?}", e);
        }
    }

    let _ = Rc::clone(&on_disconnect);
}

// To be used with the `chrome.runtime.onMessage` event
pub async fn runtime_on_message(extension: Arc<Extension>, message: JsValue, sender: JsValue) {
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
            if let Some(method) = &p.method {
                match method.as_str() {
                    "frame_summon" => {
                        unimplemented!("handle frame_summon request - {:?}", p);
                    }
                    "embedded_action_res" => {
                        if let Some(params) = &p.params {
                            if let (Some(action), Some(res)) = (params.get(0), params.get(1)) {
                                if action.get("type")
                                    == Some(&Value::String("getChainId".to_string()))
                                {
                                    if let Some(chain_id_value) = res.get("chainId") {
                                        if let Some(chain_id_str) = chain_id_value.as_str() {
                                            if let Ok(chain_id) = chain_id_str.parse::<u32>() {
                                                extension
                                                    .state
                                                    .lock()
                                                    .await
                                                    .set_current_chain(chain_id);
                                                return;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        warn!("Failed to handle embedded_action_res: {:?}", p);
                        return;
                    }
                    _ => {}
                }
            }

            // Use the provider within the extension to create the request task
            if let Some(provider) = extension.provider.as_ref() {
                let fut = create_request_task(p.clone(), provider.clone(), sender.clone());
                if provider.is_connected().await {
                    spawn_local(fut);
                } else {
                    debug!("Provider is not connected, buffering request");

                    let uuid = uuid::Uuid::new_v4().to_string();
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
                                    error: Some(Value::String("Request timed out".to_string())),
                                };
                                let message =
                                    MessagePayload::JsonResponse(eth_payload).to_js_value();
                                if let Some(tab_id) = tab_id_from_sender(sender_clone) {
                                    if let Err(e) = send_message_to_tab(tab_id, message).await {
                                        warn!("Failed to send response to tab: {:?}", e);
                                    }
                                }
                            }
                        });
                    });

                    let mut state = extension.state.lock().await;
                    state.buffered_requests.insert(
                        uuid,
                        BufferedRequest {
                            timer,
                            future: Box::pin(fut),
                        },
                    );
                }
            } else {
                warn!("Provider not initialized.");
            }
        }
    }
}

fn create_request_task(
    mut p: EthPayload,
    provider: Arc<Provider>,
    sender: JsValue,
) -> impl Future<Output = ()> {
    async move {
        warn!("Origin upstreaming not implemented: {:?}", p.base.origin);

        let method = p.method.clone().unwrap_or_default();
        let params = p.params.clone().unwrap_or_default();
        match provider.request::<Value>(&method, params).await {
            Ok(response) => p.result = Some(response),
            Err(_) => p.error = Some(Value::String("Client not available".to_string())),
        };

        send_response(sender, p).await;
    }
}

async fn send_response(sender: JsValue, eth_payload: EthPayload) {
    let message = MessagePayload::JsonResponse(eth_payload).to_js_value();
    trace!("Sending response: {:?}", message);

    if let Some(tab_id) = tab_id_from_sender(sender.clone()) {
        if let Err(e) = send_message_to_tab(tab_id, message).await {
            warn!("Failed to send response to tab {}: {:?}", tab_id, e);
        }
    }
}

fn tab_id_from_sender(sender: JsValue) -> Option<u32> {
    Reflect::get(&sender, &JsValue::from_str("tab"))
        .and_then(|tab| Reflect::get(&tab, &JsValue::from_str("id")))
        .ok()
        .and_then(|id| id.as_f64().map(|id| id as u32))
}
