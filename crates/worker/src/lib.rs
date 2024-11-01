#![feature(async_closure)]

use std::{cell::RefCell, rc::Rc};

use chrome_sys::{
    action::{self, IconPath, PopupDetails, TabIconDetails},
    alarms::{self},
    tabs::{self, Query},
};
use events::{send_event, setup_listeners};
use futures::lock::Mutex;
use js_sys::Reflect;
use jsonrpsee::wasm_client::{Client, WasmClientBuilder};
use serde_wasm_bindgen::from_value;
use state::ExtensionState;
use tracing::{debug, info, trace, warn};
use url::Url;
use wasm_bindgen::prelude::*;

extern crate console_error_panic_hook;

// mod singleton;
// use singleton::Singleton;
mod events;
mod state;
mod subscription;

const EXTENSION_PORT_NAME: &str = "frame_connect";
const CLIENT_STATUS_ALARM_KEY: &str = "check-client-status";

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

    let extension = Rc::new(RefCell::new(Extension::new().await));
    let provider = extension.borrow().provider.clone();

    trace!("Setting up event listeners");
    setup_listeners(extension.clone(), provider.unwrap().clone());

    info!("Extension initialized successfully");
    Ok(true.into())
}

pub struct Extension {
    state: Mutex<ExtensionState>,
    pub provider: Option<Rc<RefCell<Client>>>,
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

        let provider = Self::create_provider()
            .await
            .map(|client| Rc::new(RefCell::new(client)));

        let extension = Self {
            state: Mutex::new(state),
            provider: provider.ok(),
        };

        extension
    }

    async fn create_provider() -> Result<Client, JsValue> {
        let provider = WasmClientBuilder::default()
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

    async fn init_provider(&mut self) {
        if self.provider.is_some() {
            warn!("Provider already initialized");
            return;
        }

        match WasmClientBuilder::default()
            .build("ws://127.0.0.1:1248")
            .await
        {
            Ok(client) => {
                self.provider = Some(Rc::new(RefCell::new(client)));
                self.state.lock().await.set_frame_connected(true);
                debug!("Provider initialized successfully");
            }
            Err(e) => {
                // If building the client fails, initialize PROVIDER with None
                self.provider = None;
                warn!(error = ?e, "Failed to initialize JSON-RPC client");
            }
        }

        send_event("connect", None, tabs::Query::default()).await;
    }

    async fn destroy_provider(&mut self) {
        if self.provider.take().is_some() {
            self.state.lock().await.set_frame_connected(false);
            debug!("Provider destroyed");
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
