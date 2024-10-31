use chrome_sys::{
    action::{self, IconPath, PopupDetails, TabIconDetails},
    alarms::{self},
    port,
    tabs::{self, send_message_to_tab, Query},
};
use events::send_event;
use ferris_primitives::{EmbeddedAction, EmbeddedActionPayload, EthPayload, MessagePayload};
use js_sys::Reflect;
use jsonrpsee::{
    core::client::ClientT,
    wasm_client::{Client, WasmClientBuilder},
};
use once_cell::unsync::OnceCell;
use serde::Deserialize;
use serde_wasm_bindgen::from_value;
use state::ExtensionState;
use std::{cell::RefCell, rc::Rc};
use std::{
    ops::{Deref, DerefMut},
    panic,
};
use tracing::{debug, error, info, trace, warn};
use url::Url;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
extern crate console_error_panic_hook;

mod events;
mod state;
mod subscription;

const EXTENSION_PORT_NAME: &str = "frame_connect";
const CLIENT_STATUS_ALARM_KEY: &str = "check-client-status";

// Define a wrapper type around `OnceCell<Rc<RefCell<Extension>>>`
pub struct SyncOnceCell(OnceCell<Rc<RefCell<Extension>>>);

// Implement `Deref` and `DerefMut` for convenient access to the inner `OnceCell`
impl Deref for SyncOnceCell {
    type Target = OnceCell<Rc<RefCell<Extension>>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SyncOnceCell {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// Implement `Sync` unsafely for `SyncOnceCell`, as we're in a single-threaded WASM context
unsafe impl Sync for SyncOnceCell {}

// Define the singleton instance using the new wrapper type
static INSTANCE: SyncOnceCell = SyncOnceCell(OnceCell::new());

// Initializes the singleton instance asynchronously
#[wasm_bindgen]
pub async fn initialize_extension() -> Result<JsValue, JsValue> {
    // Set up a panic hook to log errors
    panic::set_hook(Box::new(console_error_panic_hook::hook));

    // Initialise tracing for logging to the console
    tracing_wasm::set_as_global_default_with_config(
        tracing_wasm::WASMLayerConfigBuilder::new()
            .set_max_level(tracing::Level::TRACE)
            .build(),
    );

    trace!("Starting extension initialization");

    // Check if INSTANCE is already initialized
    if INSTANCE.get().is_some() {
        info!("Extension already initialized");
        return Err(JsValue::null());
    }

    // Initialize INSTANCE with async extension setup using async new()
    let extension = Extension::new().await;
    INSTANCE
        .set(Rc::new(RefCell::new(extension)))
        .map_err(|_| {
            error!("Failed to set singleton instance");
            JsValue::null()
        })?;

    trace!("Setting up event listeners");
    events::setup_listeners();

    trace!("Initializing on-disconnect closure");
    ExtensionState::init_on_disconnect_closure();

    info!("Extension initialized successfully");
    Ok(true.into())
}

// Function to retrieve the singleton instance
pub fn get_extension() -> Rc<RefCell<Extension>> {
    INSTANCE.get().expect("Extension not initialized").clone()
}

pub struct Extension {
    state: ExtensionState,
    provider: Option<Client>,
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

        let mut extension = Self {
            state,
            provider: None,
        };

        extension.init_provider().await;

        extension
    }

    async fn init_provider(&mut self) {
        match WasmClientBuilder::default()
            .build("ws://127.0.0.1:1248")
            .await
        {
            Ok(client) => {
                // get_extension().borrow_mut().provider = Some(client);
                self.provider = Some(client);
                self.state.set_frame_connected(true);
                debug!("Provider initialised successfully");
            }
            Err(e) => warn!(error = ?e, "Failed to initialise JSON-RPC client"),
        }

        send_event("connect", None, tabs::Query::default()).await;
    }

    fn destroy_provider(&mut self) {
        if self.provider.take().is_some() {
            self.state.set_frame_connected(false);
            debug!("Provider destroyed");
        }
    }

    // Event handlers

    // To be used with the `chrome.runtime.onConnect` event
    pub async fn runtime_on_connect(&self, js_port: JsValue) {
        // Retrieve port name, logging on error
        trace!("Received connection: {:?}", js_port);

        #[derive(Deserialize)]
        struct Port {
            pub name: String,
        }

        let port: Port = from_value(js_port.clone()).unwrap();

        if port.name == EXTENSION_PORT_NAME {
            trace!("Connection is for frame_connect");
            get_extension().borrow_mut().state.settings_panel = Some(js_port.clone());
            self.state.update_settings_panel();

            // Add onDisconnect listener
            if port::port_add_on_disconnect_listener(
                js_port,
                self.state
                    .on_disconnect_closure
                    .as_ref()
                    .map(|c| c.as_ref().unchecked_ref())
                    .expect("on_disconnect_closure missing"),
            )
            .is_err()
            {
                tracing::error!("Failed to add onDisconnect listener");
            }
        }
    }

    // To be used with the `chrome.runtime.onMessage` event
    pub async fn runtime_on_message(&self, message: JsValue, sender: JsValue) {
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
                if let Some(provider) = &self.provider {
                    let response: Result<serde_json::Value, _> = provider
                        .request(
                            &p.method.clone().unwrap_or_default(),
                            p.params.clone().unwrap_or_default(),
                        )
                        .await;

                    trace!("Received response: {:?}", response);

                    // If the response is successful, populate `result` in EthPayload,
                    // otherwise populate `error`
                    let eth_payload = match response {
                        Ok(result) => EthPayload {
                            base: p.base,
                            method: p.method.clone(),
                            params: p.params.clone(),
                            result: Some(result),
                            error: None,
                        },
                        Err(e) => {
                            EthPayload {
                            base: p.base,
                            method: p.method.clone(),
                            params: p.params.clone(),
                            result: None,
                            error: None,
                        }},
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
            self.destroy_provider();
            self.init_provider().await;
        }
    }

    // To be used with the `chrome.tabs.onRemoved` event
    pub fn tabs_on_removed(&self, tab_id: u32) {
        get_extension().borrow_mut().state.tab_origins.remove(&tab_id);
        get_extension().borrow_mut().state.tab_unsubscribe(tab_id).unwrap_or_else(|e| {
            warn!("Failed to unsubscribe tab {}: {:?}", tab_id, e);
        });
    }

    // Handler for `chrome.tabs.onUpdated` event
    pub fn tabs_on_updated(&self, tab_id: JsValue, change_info: JsValue) {
        trace!("Received tab update event: {:?}", change_info);
        let change_info: tabs::ChangeInfo = from_value(change_info).unwrap();
        let tab_id: u32 = tab_id.as_f64().unwrap() as u32;

        // Trace tab update and check for URL changes
        trace!(tab_id, ?change_info.url, "Tab updated");

        if let Some(url) = change_info.url {
            let origin = origin_from_url(Some(url.clone()));
            debug!(tab_id, ?origin, "Updated tab origin");
            get_extension().borrow_mut().state.tab_origins.insert(tab_id, origin);

            // Attempt to unsubscribe the tab and log if it fails
            if let Err(e) = self.state.tab_unsubscribe(tab_id) {
                warn!(tab_id, error = ?e, "Failed to unsubscribe tab");
            }
        } else {
            trace!(tab_id, "No URL change detected for tab");
        }
    }

    // Handler for `chrome.tabs.onActivated` event
    pub async fn tabs_on_activated(&self, active_info: JsValue) {
        let active_info: tabs::ActiveInfo = from_value(active_info).unwrap();

        let tab = match tabs::get(active_info.tab_id).await {
            Ok(tab) => tab,
            Err(e) => {
                warn!("Failed to get tab {}: {:?}", active_info.tab_id, e);
                return;
            }
        };

        // Update the active tab ID
        get_extension().borrow_mut().state.active_tab_id = Some(active_info.tab_id);
        debug!(active_tab_id = ?self.state.active_tab_id, "Updated active tab ID");

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

    fn set_chains(&mut self, chains: JsValue) -> Result<(), JsValue> {
        let chains_vec: Vec<String> = serde_wasm_bindgen::from_value(chains)?;

        self.state.set_chains(chains_vec);
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
