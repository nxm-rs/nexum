use std::collections::HashMap;

use chrome_sys::port;
use ferris_primitives::FrameState;
use js_sys::Function;
use serde_wasm_bindgen::to_value;
use tracing::{debug, trace, warn};
use wasm_bindgen::{prelude::Closure, JsCast, JsValue};

use crate::{get_extension, subscription::Subscription};

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

    pub fn init_on_disconnect_closure() {
        // Check if on_disconnect_closure is already set
        let ext = get_extension();
        if ext.borrow().state.on_disconnect_closure.is_none() {
            debug!("Initializing on_disconnect_closure");

            let closure = Closure::wrap(Box::new(|port: JsValue| {
                debug!("on_disconnect_closure triggered");

                let ext = get_extension();
                let mut ext = ext.borrow_mut();

                // Check if `port` matches `settings_panel`
                if ext.state.settings_panel == Some(port.clone()) {
                    debug!("Resetting settings_panel state");
                    ext.state.settings_panel = None;
                    ext.state.update_settings_panel();
                }

                // Remove listener if on_disconnect_closure exists
                if let Some(closure) = &ext.state.on_disconnect_closure {
                    if port::remove_on_disconnect_listener(
                        port.clone(),
                        closure.as_ref().unchecked_ref::<Function>(),
                    )
                    .is_err()
                    {
                        warn!(
                            "Failed to remove on_disconnect_listener for port: {:?}",
                            port
                        );
                    } else {
                        debug!("Removed on_disconnect_listener for port: {:?}", port);
                    }
                }
            }) as Box<dyn Fn(JsValue)>);

            // Store the closure in the struct for reuse
            ext.borrow_mut().state.on_disconnect_closure = Some(closure);
            debug!("on_disconnect_closure initialized and stored");
        } else {
            debug!("on_disconnect_closure already initialized; skipping");
        }
    }

    // Cleanup subscriptions when a tab is closed or navigated away
    pub fn tab_unsubscribe(&self, tab_id: u32) -> Result<(), JsValue> {
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
            get_extension().borrow_mut().state.subscriptions.remove(&key);
        }

        // Simply drop all pending payloads as the remote hasn't responded and we just ignore them
        // TODO: How to drop all requests that are inflight if using a promise model

        Ok(())
    }
}
