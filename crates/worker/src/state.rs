use std::collections::HashMap;

use chrome_sys::port;
use ferris_primitives::FrameState;
use serde_wasm_bindgen::to_value;
use tracing::{debug, trace};
use wasm_bindgen::{prelude::Closure, JsValue};

use crate::{subscription::Subscription, INSTANCE};

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
    // Closure to handle port disconnect events
    pub on_disconnect_closure: Option<Closure<dyn Fn(JsValue)>>,
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

    // Cleanup subscriptions when a tab is closed or navigated away
    pub async fn tab_unsubscribe(&self, tab_id: u32) -> Result<(), JsValue> {
        // Collect all subscriptions that the tab is subscribed to
        let subscriptions_to_unsubscribe: Vec<_> = self
            .subscriptions
            .iter()
            .filter(|(_, sub)| sub.tab_id == tab_id)
            .map(|(key, _)| key.clone())
            .collect();

        let mut ext_ref = INSTANCE.get_mut(); // Directly borrow the Option<Extension> mutably
        if let Some(extension) = ext_ref.as_mut() {
            // Lock the state to check if the settings_panel matches the disconnected port
            let mut state = extension.state.lock().await;
            // Send unsubscribe request for each relevant subscription and remove it
            for key in subscriptions_to_unsubscribe {
                // Placeholder for the unsubscribe call, e.g., `send_unsubscribe(key)`
                // You could also await an async unsubscribe function if needed.
                trace!("Unsubscribing: {:?}", key);
                state.subscriptions.remove(&key);
            }
        }
        // Simply drop all pending payloads as the remote hasn't responded and we just ignore them
        // TODO: How to drop all requests that are inflight if using a promise model

        Ok(())
    }
}
