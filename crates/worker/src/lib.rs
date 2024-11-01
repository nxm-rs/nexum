#![feature(async_closure)]
use std::{cell::RefCell, rc::Rc};

use chrome_sys::{
    action::{self, IconPath, PopupDetails, TabIconDetails},
    alarms::{self},
    port,
    tabs::{self, send_message_to_tab, Query},
};
use events::send_event;
use ferris_primitives::{EmbeddedAction, EmbeddedActionPayload, EthPayload, MessagePayload};
use futures::lock::Mutex;
use js_sys::{Function, Reflect};
use jsonrpsee::{
    core::client::{ClientT, Error as JsonRpcError},
    wasm_client::{Client, WasmClientBuilder},
};
use once_cell::sync::Lazy;
use serde::Deserialize;
use serde_wasm_bindgen::from_value;
use state::ExtensionState;
use tracing::{debug, info, trace, warn};
use url::Url;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

extern crate console_error_panic_hook;

mod singleton;
use singleton::Singleton;
mod events;
mod state;
mod subscription;

const EXTENSION_PORT_NAME: &str = "frame_connect";
const CLIENT_STATUS_ALARM_KEY: &str = "check-client-status";

// Type aliases for easier use
type ExtensionSingleton = Singleton<Extension>;
type ProviderSingleton = Singleton<Client>;

// Use Lazy to initialize the Singleton statics
pub static INSTANCE: Lazy<ExtensionSingleton> = Lazy::new(ExtensionSingleton::new);
pub static PROVIDER: Lazy<ProviderSingleton> = Lazy::new(ProviderSingleton::new);

#[wasm_bindgen]
pub async fn initialize_extension() -> Result<JsValue, JsValue> {
    // Set up a panic hook to log errors
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    // Initialize tracing for logging to the console
    tracing_wasm::set_as_global_default_with_config(
        tracing_wasm::WASMLayerConfigBuilder::new()
            .set_max_level(tracing::Level::TRACE)
            .build(),
    );

    trace!("Starting extension initialization");

    // Initialize PROVIDER if it hasn't been set
    PROVIDER.initialize(None).map_err(|_| {
        info!("Provider already initialized");
        JsValue::null()
    })?;

    // Initialize INSTANCE if it hasn't been set
    let extension = Extension::new().await;
    INSTANCE.initialize(Some(extension)).map_err(|_| {
        info!("Extension already initialized");
        JsValue::null()
    })?;

    trace!("Setting up event listeners");
    events::setup_listeners();

    info!("Extension initialized successfully");
    Ok(true.into())
}

pub struct Extension {
    state: Mutex<ExtensionState>,
}

impl Extension {
    async fn new() -> Self {
        // Query for all existing tabs
        let tabs_js = tabs::query(Query::default())
            .await
            .unwrap_or_else(|_| JsValue::undefined());

        // Convert tabs to a Vec<TabInfo>
        let tabs: Vec<tabs::Info> = from_value(tabs_js).unwrap_or_default();

        // Create a HashMap to store tab origins
        let tab_origins = tabs
            .into_iter()
            .filter_map(|tab| {
                // Check if both `id` and `url` are present
                if let (Some(id), Some(url)) = (tab.id, tab.url) {
                    Some((id, origin_from_url(Some(url))))
                } else {
                    None
                }
            })
            .collect();

        // Lock the state to update the tab origins
        let state = ExtensionState {
            tab_origins,
            ..Default::default()
        };

        // Actions

        action::set_icon(TabIconDetails {
            path: Some(IconPath::Single("icons/icon96moon.png".to_string())),
            ..Default::default()
        });

        action::set_popup(PopupDetails {
            popup: "index.html".to_string(),
            ..Default::default()
        });

        // Event listeners

        // Initialise the JSON-RPC client
        // self.init_provider().await;

        match alarms::get(CLIENT_STATUS_ALARM_KEY).await {
            Ok(Some(_)) => {}
            Ok(None) | Err(_) => {
                let alarm_info = alarms::AlarmCreateInfo {
                    delay_in_minutes: Some(0.0),
                    period_in_minutes: Some(0.5),
                    ..Default::default()
                };

                if let Err(e) = alarms::create_alarm(CLIENT_STATUS_ALARM_KEY, alarm_info).await {
                    warn!("Failed to create alarm: {:?}", e);
                }
            }
        }

        let extension = Self {
            state: Mutex::new(state),
        };

        // Initialise the provider
        extension.init_provider().await;

        extension
    }

    async fn init_provider(&self) {
        match WasmClientBuilder::default()
            .build("ws://127.0.0.1:1248")
            .await
        {
            Ok(client) => {
                // Attempt to initialize PROVIDER with Some(client)
                if PROVIDER.initialize(Some(client)).is_ok() {
                    self.state.lock().await.set_frame_connected(true);
                    debug!("Provider initialized successfully");
                } else {
                    warn!("Provider is already initialized");
                }
            }
            Err(e) => {
                // If building the client fails, initialize PROVIDER with None
                let _ = PROVIDER.initialize(None);
                warn!(error = ?e, "Failed to initialize JSON-RPC client");
            }
        }

        send_event("connect", None, tabs::Query::default()).await;
    }

    async fn destroy_provider(&self) {
        // Access PROVIDER and take the inner value if it is `Some`
        let mut provider_ref = PROVIDER.get_mut(); // Borrow mutably to access Option<Client>
        if provider_ref.take().is_some() {
            self.state.lock().await.set_frame_connected(false);
            debug!("Provider destroyed");
        }
    }

    // Event handlers

    // To be used with the `chrome.runtime.onConnect` event
    pub async fn runtime_on_connect(&mut self, js_port: JsValue) {
        // Retrieve port name, logging on error
        trace!("Received connection: {:?}", js_port);

        #[derive(Deserialize)]
        struct Port {
            pub name: String,
        }

        let port: Port = from_value(js_port.clone()).unwrap();

        if port.name == EXTENSION_PORT_NAME {
            trace!("Connection is for frame_connect");
            self.state.lock().await.settings_panel = Some(js_port.clone());
            self.state.lock().await.update_settings_panel();

            // Add onDisconnect listener
            self.port_on_disconnect(js_port.clone());
        }
    }

    // Function to set up the on_disconnect handler
    fn port_on_disconnect(&self, port: JsValue) {
        // Create a placeholder for on_disconnect, initially set to None
        let on_disconnect: Rc<RefCell<Option<Closure<dyn Fn(JsValue)>>>> =
            Rc::new(RefCell::new(None));

        // Clone references to port and on_disconnect to use in the closure
        let port_clone = port.clone();
        let on_disconnect_clone = Rc::clone(&on_disconnect);

        let on_disconnect_closure = Closure::wrap(Box::new(move |_: JsValue| {
            let port_clone_inner = port_clone.clone();
            let on_disconnect_inner = on_disconnect_clone.clone();

            // Spawn a task to handle disconnection asynchronously
            spawn_local(async move {
                info!("Port disconnected");

                // Access the singleton extension instance
                let mut ext_ref = INSTANCE.get_mut(); // Directly borrow the Option<Extension> mutably
                if let Some(extension) = ext_ref.as_mut() {
                    // Lock the state to check if the settings_panel matches the disconnected port
                    let mut state = extension.state.lock().await;
                    if state.settings_panel == Some(port_clone_inner.clone()) {
                        debug!("Resetting settings_panel state");
                        state.settings_panel = None;
                        state.update_settings_panel();
                    }
                }

                // Remove the on_disconnect listener using the original closure
                if let Some(ref closure) = *on_disconnect_inner.borrow() {
                    if port::remove_on_disconnect_listener(
                        port_clone_inner.clone(),
                        closure.as_ref().unchecked_ref::<Function>(),
                    )
                    .is_err()
                    {
                        warn!(
                            "Failed to remove onDisconnect listener for port: {:?}",
                            port_clone_inner
                        );
                    } else {
                        info!(
                            "Removed onDisconnect listener for port: {:?}",
                            port_clone_inner
                        );
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
    pub async fn runtime_on_message(message: JsValue, sender: JsValue) {
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
                // Make an upstream request
                let provider = PROVIDER.get();
                // Check if the provider has been set
                if let Some(provider) = provider.as_ref() {
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
                } else {
                    warn!("Provider not initialised");
                }
            }
        }
    }

    // To be used with the `chrome.idle.onStateChanged` event
    pub async fn idle_on_state_changed(&mut self, state: JsValue) {
        if state == "active" {
            self.destroy_provider().await;
            self.init_provider().await;
        }
    }

    // To be used with the `chrome.tabs.onRemoved` event
    pub async fn tabs_on_removed(&self, tab_id: u32) {
        self.state.lock().await.tab_origins.remove(&tab_id);
        self.state
            .lock()
            .await
            .tab_unsubscribe(tab_id)
            .await
            .unwrap_or_else(|e| {
                warn!("Failed to unsubscribe tab {}: {:?}", tab_id, e);
            });
    }

    // Handler for `chrome.tabs.onUpdated` event
    pub async fn tabs_on_updated(&mut self, tab_id: JsValue, change_info: JsValue) {
        trace!("Received tab update event: {:?}", change_info);
        let change_info: tabs::ChangeInfo = from_value(change_info).unwrap();
        let tab_id: u32 = tab_id.as_f64().unwrap() as u32;

        // Trace tab update and check for URL changes
        trace!(tab_id, ?change_info.url, "Tab updated");

        if let Some(url) = change_info.url {
            let origin = origin_from_url(Some(url.clone()));
            debug!(tab_id, ?origin, "Updated tab origin");
            self.state.lock().await.tab_origins.insert(tab_id, origin);

            // Attempt to unsubscribe the tab and log if it fails
            if let Err(e) = self.state.lock().await.tab_unsubscribe(tab_id).await {
                warn!(tab_id, error = ?e, "Failed to unsubscribe tab");
            }
        } else {
            trace!(tab_id, "No URL change detected for tab");
        }
    }

    // Handler for `chrome.tabs.onActivated` event
    pub async fn tabs_on_activated(&mut self, active_info: JsValue) {
        let active_info: tabs::ActiveInfo = from_value(active_info).unwrap();

        let tab = match tabs::get(active_info.tab_id).await {
            Ok(tab) => tab,
            Err(e) => {
                warn!("Failed to get tab {}: {:?}", active_info.tab_id, e);
                return;
            }
        };

        // Update the active tab ID
        self.state.lock().await.active_tab_id = Some(active_info.tab_id);
        debug!(active_tab_id = ?self.state.lock().await.active_tab_id, "Updated active tab ID");

        // Get and validate tab origin
        if tab.valid() {
            let message = MessagePayload::EmbeddedAction(EmbeddedActionPayload::new(
                EmbeddedAction::new("getChainId".to_string(), JsValue::NULL),
            ));

            spawn_local(async move {
                if let Err(e) =
                    tabs::send_message_to_tab(tab.id.unwrap(), message.to_js_value()).await
                {
                    warn!(
                        "Failed to send message to tab {}: {:?}",
                        active_info.tab_id, e
                    );
                }
            });
        } else {
            debug!("Filtering tab as invalid: {:?}", tab);
        }
    }

    // To be used with the `chrome.alarms.onAlarm` event
    pub fn alarms_on_alarm(&self, alarm: JsValue) {
        let alarm: alarms::AlarmInfo = from_value(alarm).unwrap();

        if alarm.name == CLIENT_STATUS_ALARM_KEY {
            warn!("Not implemented: should continually check RPC client status");
        }
    }

    async fn set_chains(&mut self, chains: JsValue) -> Result<(), JsValue> {
        let chains_vec: Vec<String> = serde_wasm_bindgen::from_value(chains)?;

        self.state.lock().await.set_chains(chains_vec);
        Ok(())
    }
}

fn origin_from_url(url: Option<String>) -> String {
    match url {
        Some(u) => {
            if let Ok(parsed_url) = Url::parse(&u) {
                parsed_url.origin().ascii_serialization()
            } else {
                String::new()
            }
        }
        None => String::new(),
    }
}

fn get_origin(sender: JsValue) -> String {
    let url = Reflect::get(&sender, &JsValue::from_str("url"))
        .ok()
        .and_then(|val| val.as_string());
    origin_from_url(url)
}
