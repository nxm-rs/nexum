use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc};

use chrome_sys::port;
use nexum_primitives::FrameState;
use serde_wasm_bindgen::to_value;
use tracing::{debug, trace};
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;

use crate::{
    subscription::{unsubscribe, Subscription},
    Extension,
};

#[derive(Default)]
pub(crate) struct ExtensionState {
    /// The active tab ID
    pub active_tab_id: Option<u32>,
    /// The Chrome port for the settings panel
    pub settings_panel: Option<JsValue>, // Holds the Chrome port for `postMessage`
    /// A mapping of the subscription ID to the subscription
    pub subscriptions: HashMap<String, Subscription>,
    /// A mapping of tab ID to the origin
    pub tab_origins: HashMap<u32, String>,
    /// The current state of the frame
    pub frame_state: FrameState,
    /// A vector of buffered upstream requests
    pub buffered_requests: HashMap<String, BufferedRequest>,
}

pub(crate) struct BufferedRequest {
    pub timer: gloo_timers::callback::Timeout,
    pub future: Pin<Box<dyn Future<Output = ()>>>,
}

impl ExtensionState {
    pub fn update_settings_panel(&self) {
        if let Some(panel) = &self.settings_panel {
            debug!("Updating settings panel with new frame state");

            let frame_state_js: JsValue = to_value(&self.frame_state).unwrap();
            port::post_message(&panel, frame_state_js).unwrap();
        } else {
            debug!("No settings panel available to update");
        }
    }

    pub fn set_chains(&mut self, chains: Vec<String>) {
        debug!("Setting available chains: {:?}", chains);
        self.frame_state.available_chains = chains;
        self.update_settings_panel();
    }

    pub fn set_current_chain(&mut self, chain_id: u32) {
        debug!("Setting current chain: {}", chain_id);
        self.frame_state.current_chain = Some(chain_id);
        self.update_settings_panel();
    }

    pub fn set_frame_connected(&mut self, connected: bool) {
        debug!("Setting frame connected: {}", connected);
        self.frame_state.frame_connected = connected;
        self.update_settings_panel();
    }
}

// Cleanup subscriptions when a tab is closed or navigated away
pub async fn tab_unsubscribe(extension: Arc<Extension>, tab_id: u32) -> Result<(), JsValue> {
    let mut state = extension.state.lock().await;

    let subscriptions_to_unsubscribe: Vec<_> = state
        .subscriptions
        .iter()
        .filter(|(_, sub)| sub.tab_id == tab_id)
        .map(|(key, _)| key.clone())
        .collect();

    // Send unsubscribe request for each relevant subscription and remove it
    for sub_id in subscriptions_to_unsubscribe {
        let sub_id_clone = sub_id.clone();
        spawn_local(async move {
            trace!("Unsubscribing: {:?}", sub_id_clone);
            if let Err(e) = unsubscribe(sub_id_clone).await {
                trace!("Failed to unsubscribe: {:?}", e);
            }
        });
        state.subscriptions.remove(&sub_id);
    }

    Ok(())
}
