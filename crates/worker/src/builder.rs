use std::sync::Arc;

use chrome_sys::{
    action::{self, IconPath, PopupDetails, TabIconDetails},
    alarms,
    tabs::{self, Query},
};
use futures::lock::Mutex;
use serde_wasm_bindgen::from_value;
use tracing::{info, warn};
use wasm_bindgen::JsValue;

use crate::{origin_from_url, state::ExtensionState, Extension, Provider, CLIENT_STATUS_ALARM_KEY};

pub struct ExtensionBuilder {
    state: Option<Arc<Mutex<ExtensionState>>>,
    provider: Option<Arc<Provider>>,
}

impl ExtensionBuilder {
    pub fn new() -> Self {
        Self {
            state: None,
            provider: None,
        }
    }

    /// Adds a `Provider` to the builder
    pub fn with_provider(mut self, provider: Arc<Provider>) -> Self {
        self.provider = Some(provider);
        self
    }

    /// Builds the `Extension` instance with configured `Provider` and `ExtensionState`
    pub async fn build(mut self) -> Result<Arc<Extension>, JsValue> {
        // Initialize ExtensionState
        let tabs_js = tabs::query(Query::default())
            .await
            .unwrap_or_else(|_| JsValue::undefined());
        let tabs: Vec<tabs::Info> = from_value(tabs_js).unwrap_or_default();
        let tab_origins = tabs
            .into_iter()
            .filter_map(|tab| {
                if let (Some(id), Some(url)) = (tab.id, tab.url) {
                    Some((id, origin_from_url(Some(url))))
                } else {
                    None
                }
            })
            .collect();

        // Create the state with initial tab origins
        let state = ExtensionState {
            tab_origins,
            ..Default::default()
        };
        self.state = Some(Arc::new(Mutex::new(state)));

        // Set icon and popup actions
        let _ = action::set_icon(TabIconDetails {
            path: Some(IconPath::Single("icons/icon96moon.png".to_string())),
            ..Default::default()
        });
        let _ = action::set_popup(PopupDetails {
            popup: "index.html".to_string(),
            ..Default::default()
        });

        // Set up the alarm if not already set
        match alarms::get(CLIENT_STATUS_ALARM_KEY).await {
            Ok(Some(_)) => {}
            Ok(None) | Err(_) => {
                let alarm_info = alarms::AlarmCreateInfo {
                    delay_in_minutes: Some(0.0),
                    period_in_minutes: Some(0.5),
                    ..Default::default()
                };

                info!("Creating alarm: {:?}", alarm_info);

                if let Err(e) = alarms::create_alarm(CLIENT_STATUS_ALARM_KEY, alarm_info).await {
                    warn!("Failed to create alarm: {:?}", e);
                }
            }
        }

        // Create the `Extension` with the state and an uninitialized provider
        let extension = Arc::new(Extension {
            state: self.state.clone().unwrap(),
            provider: None,
        });

        // Initialize the provider with a reference to the extension and set it
        let provider = Provider::new(extension.clone()); // `Provider::new` already returns `Arc<Provider>`
        provider.init().await;

        // Here, instead of trying to mutate the `Arc`, we recreate it with the provider included
        let extension_with_provider = Arc::new(Extension {
            state: extension.state.clone(),
            provider: Some(provider),
        });

        Ok(extension_with_provider)
    }
}
