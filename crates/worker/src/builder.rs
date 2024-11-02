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

use crate::{
    origin_from_url, state::ExtensionState, Extension, ProviderType, CLIENT_STATUS_ALARM_KEY,
};

pub struct ExtensionBuilder {
    provider: Option<ProviderType>,
}

impl ExtensionBuilder {
    pub fn new() -> Self {
        Self { provider: None }
    }

    pub fn with_provider(mut self, provider: ProviderType) -> Self {
        self.provider = Some(provider);
        self
    }

    pub async fn build(self) -> Extension {
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
                if let (Some(id), Some(url)) = (tab.id, tab.url) {
                    Some((id, origin_from_url(Some(url))))
                } else {
                    None
                }
            })
            .collect();

        // Initialize the ExtensionState
        let state = ExtensionState {
            tab_origins,
            ..Default::default()
        };

        // Set icon and popup actions
        let _ = action::set_icon(TabIconDetails {
            path: Some(IconPath::Single("icons/icon96moon.png".to_string())),
            ..Default::default()
        });

        let _ = action::set_popup(PopupDetails {
            popup: "index.html".to_string(),
            ..Default::default()
        });

        // Set up alarm if not already set
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

        // Return the constructed Extension
        Extension {
            state: Arc::new(Mutex::new(state)),
            provider: self.provider,
        }
    }
}
