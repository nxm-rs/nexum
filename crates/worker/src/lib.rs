use chrome_sys::{
    get_alarm, port_add_on_disconnect_listener, port_post_message,
    port_remove_on_disconnect_listener, send_message_to_tab, IconPath, PopupDetails, QueryInfo,
    TabIconDetails,
};
use futures::lock::Mutex;
use js_sys::{Function, Reflect};
use jsonrpsee::{
    core::client::ClientT,
    wasm_client::{Client, WasmClientBuilder},
};
use serde::{Deserialize, Serialize};
use serde_json::{self, Value};
use serde_wasm_bindgen::{from_value, to_value};
use std::{collections::HashMap, sync::Arc};
use tracing::{debug, trace, warn};
use url::Url;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

const EXTENSION_PORT_NAME: &str = "frame_connect";
const CLIENT_STATUS_ALARM_KEY: &str = "check-client-status";

#[derive(Serialize, Deserialize, Default)]
pub struct FrameState {
    pub frame_connected: bool,
    pub available_chains: Vec<String>,
    pub current_chain: Option<u32>,
}

#[derive(Serialize, Deserialize)]
pub enum SubscriptionType {
    ChainChanged,
    ChainsChanged,
    AccountsChanged,
    NetworkChanged,
    Message,
    Unknown,
}

#[derive(Serialize, Deserialize)]
pub struct Subscription {
    pub tab_id: u32,
    pub type_: SubscriptionType,
}

pub fn sub_type(params: &JsValue) -> SubscriptionType {
    match serde_wasm_bindgen::from_value::<Value>(params.clone()) {
        Ok(Value::Array(params_vec)) => {
            if let Some(Value::String(sub_type_str)) = params_vec.get(0) {
                match sub_type_str.as_str() {
                    "ChainChanged" => SubscriptionType::ChainChanged,
                    "ChainsChanged" => SubscriptionType::ChainsChanged,
                    "AccountsChanged" => SubscriptionType::AccountsChanged,
                    "NetworkChanged" => SubscriptionType::NetworkChanged,
                    "Message" => SubscriptionType::Message,
                    _ => SubscriptionType::Unknown,
                }
            } else {
                warn!("First parameter is not a string");
                SubscriptionType::Unknown
            }
        }
        Ok(_) => {
            warn!("Params is not an array");
            SubscriptionType::Unknown
        }
        Err(_) => {
            warn!("Error parsing params as JSON array");
            SubscriptionType::Unknown
        }
    }
}

#[derive(Serialize, Deserialize)]
struct PendingPayload {
    pub tab_id: u32,
    pub payload_id: u32,
    pub method: String,
    #[serde(skip_serializing, skip_deserializing)]
    pub params: JsValue,
    pub origin: String,
}

impl PendingPayload {
    pub fn new(
        tab_id: u32,
        payload_id: u32,
        method: String,
        params: JsValue,
        origin: String,
    ) -> Self {
        Self {
            tab_id,
            payload_id,
            method,
            params,
            origin,
        }
    }

    // Converts params to JSON string when needed
    pub fn get_params_as_json(&self) -> Result<String, serde_wasm_bindgen::Error> {
        let json_value: serde_json::Value = serde_wasm_bindgen::from_value(self.params.clone())?;
        serde_json::to_string(&json_value)
            .map_err(|e| serde_wasm_bindgen::Error::new(e.to_string().as_str()))
    }

    // Converts JSON string back to JsValue for params
    pub fn set_params_from_json(
        &mut self,
        json_str: &str,
    ) -> Result<(), serde_wasm_bindgen::Error> {
        let json_value: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|_| serde_wasm_bindgen::Error::new("Failed to parse JSON string"))?;
        self.params = serde_wasm_bindgen::to_value(&json_value)?;
        Ok(())
    }
}

#[derive(Default)]
struct ExtensionState {
    /// The active tab ID
    active_tab_id: Option<u32>,
    /// The Chrome port for the settings panel
    settings_panel: Option<JsValue>, // Holds the Chrome port for `postMessage`
    /// A mapping of the subscription ID to the subscription
    subscriptions: HashMap<String, Subscription>,
    /// A mapping of the RPC request ID to the pending payload
    pending: HashMap<u32, PendingPayload>,
    /// A mapping of tab ID to the origin
    tab_origins: HashMap<u32, String>,
    /// The current state of the frame
    frame_state: FrameState,
    /// Closure to handle port disconnect events
    on_disconnect_closure: Option<Closure<dyn Fn(JsValue)>>,
}

impl ExtensionState {
    pub fn new() -> Self {
        // Initialise tracing for logging to the console
        tracing_wasm::set_as_global_default_with_config(
            tracing_wasm::WASMLayerConfigBuilder::new()
                .set_max_level(tracing::Level::TRACE)
                .build(),
        );

        Self {
            active_tab_id: None,
            settings_panel: None,
            pending: HashMap::new(),
            subscriptions: HashMap::new(),
            tab_origins: HashMap::new(),
            frame_state: FrameState {
                frame_connected: false,
                available_chains: vec![],
                current_chain: None,
            },
            on_disconnect_closure: None,
        }
    }

    pub fn update_settings_panel(&self) {
        if let Some(panel) = &self.settings_panel {
            let state_js = to_value(&self.frame_state).expect("Failed to serialize frame state");
            port_post_message(panel.clone(), state_js)
                .expect("Failed to call postMessage on settings panel");
        }
    }

    pub fn set_chains(&mut self, chains: Vec<String>) {
        self.frame_state.available_chains = chains;
        self.update_settings_panel();
    }

    pub fn set_current_chain(&mut self, chain_id: u32) {
        self.frame_state.current_chain = Some(chain_id);
        self.update_settings_panel();
    }

    pub fn set_frame_connected(&mut self, connected: bool) {
        self.frame_state.frame_connected = connected;
        self.update_settings_panel();
    }

    pub async fn init_on_disconnect_closure(self_arc: Arc<Mutex<Self>>) {
        // Lock self to access and potentially initialize the closure
        let mut state = self_arc.lock().await;

        // Initialize the closure only if it hasn't been set up yet
        if state.on_disconnect_closure.is_none() {
            let ext_state = Arc::clone(&self_arc); // Clone the Arc<Mutex<Self>> to use inside the closure

            let closure = Closure::wrap(Box::new(move |port: JsValue| {
                let ext_state = Arc::clone(&ext_state); // Clone again for the async move block

                spawn_local(async move {
                    let mut state = ext_state.lock().await;

                    // Check if this `port` matches `settings_panel`
                    if state.settings_panel == Some(port.clone()) {
                        state.settings_panel = None;
                        state.update_settings_panel();
                    }

                    // Remove listener from this port, if necessary
                    let self_lock = ext_state.lock().await;
                    if let Some(ref closure) = self_lock.on_disconnect_closure {
                        port_remove_on_disconnect_listener(
                            port,
                            closure.as_ref().unchecked_ref::<Function>(),
                        );
                    }
                });
            }) as Box<dyn Fn(JsValue)>);

            // Store the closure in the struct for reuse
            state.on_disconnect_closure = Some(closure);
        }
    }

    // Cleanup subscriptions when a tab is closed or navigated away
    fn tab_unsubscribe(&mut self, tab_id: u32) -> Result<(), JsValue> {
        // Collect all subscriptions that the tab is subscribed to
        let subscriptions_to_unsubscribe: Vec<_> = self
            .subscriptions
            .iter()
            .filter(|(_, sub)| sub.tab_id == tab_id)
            .map(|(key, _)| key.clone())
            .collect();

        // Send unsubscribe request for each relevant subscription and remove it
        for key in subscriptions_to_unsubscribe {
            // Placeholder for the unsubscribe call, e.g., `send_unsubscribe(key)`
            // You could also await an async unsubscribe function if needed.
            trace!("Unsubscribing: {:?}", key);
            self.subscriptions.remove(&key);
        }

        // Simply drop all pending payloads as the remote hasn't responded and we just ignore them
        self.pending.retain(|_, payload| payload.tab_id != tab_id);

        Ok(())
    }
}

#[wasm_bindgen]
pub struct Extension {
    state: Arc<Mutex<ExtensionState>>,
    provider: Option<Arc<Mutex<Client>>>,
}

#[wasm_bindgen]
impl Extension {
    #[wasm_bindgen(constructor)]
    pub async fn new() -> Self {
        // Query for all existing tabs
        let tabs_js = chrome_sys::query_tabs(QueryInfo::default())
            .await
            .unwrap_or_else(|_| JsValue::undefined());

        // Convert tabs to a Vec<TabInfo>
        let tabs: Vec<TabInfo> = from_value(tabs_js).unwrap_or_default();

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

        let mut extension = Self {
            state: Arc::new(Mutex::new(ExtensionState {
                tab_origins,
                ..ExtensionState::default()
            })),
            provider: None,
        };

        let ext_state = Arc::clone(&extension.state);
        spawn_local(async move {
            ExtensionState::init_on_disconnect_closure(ext_state).await;
        });

        extension.set_icon(TabIconDetails {
            path: Some(IconPath::Single("icons/icon96moon.png".to_string())),
            ..Default::default()
        }).expect("Failed to set icon");

        extension.set_popup(PopupDetails {
            popup: "index.html".to_string(),
            ..Default::default()
        }).expect("Failed to set popup");

        // Initialise the provider
        extension.init_provider().await;

        // Start alarms
        let alarm = get_alarm(CLIENT_STATUS_ALARM_KEY)
            .await
            .unwrap_or_else(|_| JsValue::undefined());

        if alarm.is_undefined() {
            let alarm_info = AlarmCreateInfo {
                delay_in_minutes: Some(0.0),
                period_in_minutes: Some(0.5),
                ..Default::default()
            };

            if let Err(e) =
                chrome_sys::create_alarm(CLIENT_STATUS_ALARM_KEY, to_value(&alarm_info).unwrap())
                    .await
            {
                warn!("Failed to create alarm: {:?}", e);
            };
        }

        extension
    }

    async fn init_provider(&mut self) {
        match WasmClientBuilder::default().build("ws://172.20.0.5:8545").await {
            Ok(client) => {
                self.provider = Some(Arc::new(Mutex::new(client)));
                self.state.lock().await.set_frame_connected(true);
                debug!("Provider initialised successfully");
            },
            Err(e) => warn!(error = ?e, "Failed to initialise JSON-RPC client"),
        }
    }

    async fn destroy_provider(&mut self) {
        if self.provider.take().is_some() {
            self.state.lock().await.set_frame_connected(false);
            debug!("Provider destroyed");
        }
    }

    // Event handlers

    // To be used with the `chrome.runtime.onConnect` event
    pub async fn runtime_on_connect(&mut self, port: &JsValue) {
        let ext_state = Arc::clone(&self.state);

        // Use error handling on Reflect::get
        let port_name = Reflect::get(port, &JsValue::from("name")).unwrap_or_else(|_| {
            tracing::error!("Failed to retrieve port name");
            JsValue::UNDEFINED
        });

        if port_name == EXTENSION_PORT_NAME {
            let mut state = ext_state.lock().await;
            state.settings_panel = Some(port.clone());
            state.update_settings_panel();

            // Add the onDisconnect listener using the stored closure
            if let Err(_) = port_add_on_disconnect_listener(
                port.clone(),
                state
                    .on_disconnect_closure
                    .as_ref()
                    .unwrap()
                    .as_ref()
                    .unchecked_ref(),
            ) {
                tracing::error!("Failed to add onDisconnect listener");
            }
        }
    }

    // To be used with the `chrome.runtime.onMessage` event
    pub async fn runtime_on_message(&self, message: JsValue, sender: JsValue) {
        tracing::trace!("Received message: {:?}", message);
        tracing::warn!("Not implemented: process_message");
    }

    // To be used with the `chrome.idle.onStateChanged` event
    pub async fn idle_on_state_changed(&mut self, state: JsValue) {
        if state == "active" {
            self.destroy_provider().await;
            self.init_provider().await;
        }
    }

    // To be used with the `chrome.tabs.onRemoved` event
    pub fn tabs_on_removed(&self, tab_id: u32) {
        let ext_state = Arc::clone(&self.state);

        spawn_local(async move {
            let mut state = ext_state.lock().await;
            state.tab_origins.remove(&tab_id);
            state.tab_unsubscribe(tab_id).unwrap_or_else(|e| {
                warn!("Failed to unsubscribe tab {}: {:?}", tab_id, e);
            });
        });
    }

    // To be used with the `chrome.tabs.onUpdated` event
    pub fn tabs_on_updated(&self, tab_id: u32, change_info: JsValue) {
        let ext_state = Arc::clone(&self.state);

        spawn_local(async move {
            let mut state = ext_state.lock().await;

            // Get the change info
            let change_info: ChangeInfo = match from_value(change_info) {
                Ok(info) => info,
                Err(e) => {
                    warn!("Failed to parse change info: {:?}", e);
                    return;
                }
            };

            // Update the tab origin if the URL has changed
            if let Some(url) = change_info.url {
                state.tab_origins.insert(tab_id, origin_from_url(Some(url)));
                state.tab_unsubscribe(tab_id).unwrap_or_else(|e| {
                    warn!("Failed to unsubscribe tab {}: {:?}", tab_id, e);
                });
            }
        });
    }

    // To be used with the `chrome.tabs.onActivated` event
    pub async fn tabs_on_activated(&self, active_info: JsValue) {
        let ext_state = Arc::clone(&self.state);

        let active_info: TabActiveInfo = match from_value(active_info) {
            Ok(info) => info,
            Err(e) => {
                warn!("Failed to parse active info: {:?}", e);
                return;
            }
        };

        let tab = chrome_sys::get_tab(active_info.tab_id)
            .await
            .unwrap_or_else(|e| {
                warn!("Failed to get tab {}: {:?}", active_info.tab_id, e);
                JsValue::UNDEFINED
            });
        let tab: TabInfo = from_value(tab).unwrap();

        spawn_local(async move {
            let mut state = ext_state.lock().await;

            // First, update the active tab id
            state.active_tab_id = Some(active_info.tab_id);
            debug!("Set active tab ID: {:?}", active_info.tab_id);

            // drop the lock to prevent deadlock
            drop(state);

            // Get the tab origin
            let origin = Url::parse(&tab.url.unwrap()).unwrap();

            if origin.scheme() == "http" || origin.scheme() == "file" {
                #[derive(Serialize, Deserialize, Debug)]
                #[serde(rename_all = "camelCase")]
                pub enum EmbeddedActionType {
                    #[serde(rename = "embedded:action")]
                    EmbeddedAction,
                }

                #[derive(Serialize, Deserialize, Debug)]
                pub struct Action {
                    #[serde(rename = "type")]
                    action_type: String,
                }

                #[derive(Serialize, Deserialize, Debug)]
                pub struct EmbeddedAction {
                    #[serde(rename = "type")]
                    action_type: EmbeddedActionType,
                    action: Action,
                }

                let message = EmbeddedAction {
                    action_type: EmbeddedActionType::EmbeddedAction,
                    action: Action {
                        action_type: "getChainId".to_string(),
                    },
                };

                send_message_to_tab(tab.id.unwrap(), to_value(&message).unwrap())
                    .await
                    .expect("Send message failed");
            }
        });
    }

    // To be used with the `chrome.alarms.onAlarm` event
    pub async fn alarms_on_alarm(&self, alarm: JsValue) {
        let alarm: AlarmInfo = from_value(alarm).unwrap();

        if alarm.name == CLIENT_STATUS_ALARM_KEY {
            warn!("Not implemented: should continually check RPC client status");
        }
    }

    async fn set_chains(&self, chains: JsValue) -> Result<(), JsValue> {
        let chains_vec: Vec<String> = serde_wasm_bindgen::from_value(chains)?;

        let ext_state = Arc::clone(&self.state);
        spawn_local(async move {
            let mut state = ext_state.lock().await;
            state.set_chains(chains_vec);
        });
        Ok(())
    }

    // Sets the icon for the Chrome extension
    fn set_icon(&self, details: TabIconDetails) -> Result<(), JsValue> {
        chrome_sys::set_icon(details)
    }

    // Sets the popup for the Chrome extension
    fn set_popup(&self, details: PopupDetails) -> Result<(), JsValue> {
        chrome_sys::set_popup(details)
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

// Define EthEvent struct similar to the EthEvent TypeScript type
#[derive(Serialize)]
struct EthEvent<'a> {
    #[serde(rename = "type")]
    event_type: &'static str,
    event: &'a str,
    args: serde_json::Value,
}

// Send an event to a specific tab
async fn send_event_to_tab(tab_id: u32, event: &'static str, args: JsValue) -> Result<(), JsValue> {
    // Convert `args` to `serde_json::Value`
    let args: serde_json::Value = from_value(args).unwrap_or(serde_json::Value::Null);

    let event_payload = EthEvent {
        event_type: "eth:event",
        event,
        args,
    };

    let event_payload_js = to_value(&event_payload).map_err(|e| {
        warn!("Failed to serialize event payload: {:?}", e);
        JsValue::from_str("Failed to serialize event payload")
    })?;

    chrome_sys::send_message_to_tab(tab_id, event_payload_js)
        .await
        .map_err(|e| {
            warn!(
                "Error sending event \"{}\" to tab {}: {:?}",
                event, tab_id, e
            );
            JsValue::from_str("Error sending message to tab")
        })?;

    Ok(())
}

#[derive(Deserialize)]
struct TabInfo {
    id: Option<u32>, // Tab ID is f64 due to JS number format
    url: Option<String>,
}

#[derive(Deserialize)]
struct TabActiveInfo {
    tab_id: u32,
    window_id: u32,
}

#[derive(Deserialize)]
struct AlarmInfo {
    period_in_minutes: Option<f64>,
    scheduled_time: f64,
    name: String,
}

#[derive(Serialize, Deserialize, Default)]
struct AlarmCreateInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delay_in_minutes: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub period_in_minutes: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when: Option<f64>,
}

#[derive(Serialize, Deserialize)]
struct ChangeInfo {
    pub url: Option<String>,
}

// Generalized send_event function to handle any array type for args
async fn send_event(
    event: &'static str,
    args: Option<JsValue>, // Pass JsValue directly, defaulting to empty object if None
    selector: QueryInfo,
) {
    let tabs_result = chrome_sys::query_tabs(selector)
        .await
        .map_err(|e| warn!("Failed to query tabs: {:?}", e));

    if let Ok(tabs_js) = tabs_result {
        spawn_local(async move {
            let tabs: Vec<TabInfo> = from_value(tabs_js).unwrap_or_default();

            // Default args to an empty JavaScript object if None
            let args_js = args.unwrap_or_else(|| JsValue::undefined());

            // Filter and process tabs with valid `id` and `url`
            for tab in tabs
                .iter()
                .filter(|tab| tab.id.is_some() && tab.url.is_some())
            {
                if let Some(tab_id) = tab.id {
                    if let Err(e) = send_event_to_tab(tab_id as u32, event, args_js.clone()).await {
                        warn!("Failed to send event to tab {}: {:?}", tab_id, e);
                    }
                }
            }
        });
    }
}
