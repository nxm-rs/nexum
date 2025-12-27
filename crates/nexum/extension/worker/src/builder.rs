use std::sync::Arc;

use futures::lock::Mutex;
use nexum_chrome_gloo::{alarms, tabs};
use nexum_chrome_gloo::tabs::Tab;
use nexum_chrome_sys::action::{self, SetPopupDetails};
use nexum_chrome_sys::alarms::AlarmCreateInfo;
use tracing::info;
use wasm_bindgen::prelude::*;

use crate::{
    CLIENT_STATUS_ALARM_KEY, Extension, Provider, origin_from_url,
    state::{ExtensionState, set_icon_for_connection_state},
};

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
        let tabs_js = tabs::query(&tabs::QueryQueryInfo::new())
            .await
            .unwrap_or_else(|_| JsValue::undefined());
        let tab_array = js_sys::Array::from(&tabs_js);
        let tabs: Vec<Tab> = tab_array
            .iter()
            .map(|t| t.unchecked_into())
            .collect();
        let tab_origins = tabs
            .into_iter()
            .filter_map(|tab| {
                if let (Some(id), Some(url)) = (tab.get_id(), tab.get_url()) {
                    Some((id as u32, origin_from_url(Some(url))))
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

        set_icon_for_connection_state(
            &self
                .state
                .as_ref()
                .unwrap()
                .lock()
                .await
                .frame_state
                .frame_connected,
        );

        let popup_details = SetPopupDetails::new();
        popup_details.set_popup("index.html".to_string());
        let _ = action::set_popup(popup_details.into());

        // Set up the alarm if not already set
        match alarms::get(CLIENT_STATUS_ALARM_KEY).await {
            Ok(Some(_)) => {}
            Ok(None) | Err(_) => {
                let alarm_info = AlarmCreateInfo::new();
                alarm_info.set_delay_in_minutes(0.0);
                alarm_info.set_period_in_minutes(0.5);

                info!("Creating alarm");

                alarms::create(CLIENT_STATUS_ALARM_KEY, &alarm_info);
            }
        }

        // Create the `Extension` with the state and an uninitialized provider
        let extension = Arc::new(Extension {
            state: self.state.clone().unwrap(),
            provider: None,
        });

        // Initialize the provider with a reference to the extension and set it
        let provider = Provider::new(extension.clone());
        provider.init().await;

        // Here, instead of trying to mutate the `Arc`, we recreate it with the provider included
        let extension_with_provider = Arc::new(Extension {
            state: extension.state.clone(),
            provider: Some(provider),
        });

        Ok(extension_with_provider)
    }
}
