use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc};

use alloy_chains::Chain;
use nexum_chrome_gloo::runtime::port;
use nexum_chrome_sys::action::{self, SetIconDetails};
use nexum_chrome_sys::runtime::Port;
use nexum_primitives::{ConnectionState, FrameState};
use serde_wasm_bindgen::to_value;
use tracing::{debug, trace};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

use crate::{
    Extension,
    subscription::{Subscription, unsubscribe},
};

#[derive(Default)]
pub(crate) struct ExtensionState {
    /// The active tab ID
    pub active_tab_id: Option<u32>,
    /// The Chrome port for the settings panel
    pub settings_panel: Option<Port>,
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
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update_settings_panel(&self) {
        if let Some(panel) = &self.settings_panel {
            debug!("Updating settings panel with new frame state");

            let frame_state_js: JsValue = to_value(&self.frame_state).unwrap();
            let _ = port::post_message(panel, frame_state_js);
        } else {
            debug!("No settings panel available to update");
        }
    }

    pub fn set_chains(&mut self, chains: HashMap<Chain, ConnectionState>) {
        debug!("Setting available chains: {:?}", chains);
        self.frame_state.available_chains = chains;
        self.update_settings_panel();
    }

    pub fn set_current_chain(&mut self, chain: Chain) {
        debug!("Setting current chain: {}", chain);
        self.frame_state.current_chain_in_tab = Some(chain);
        self.update_settings_panel();
    }

    pub fn set_frame_connected(&mut self, connected: ConnectionState) {
        match connected.is_connected() {
            true => debug!("Provider connected"),
            false => debug!("Provider disconnected"),
        }
        self.frame_state.frame_connected = connected;
        set_icon_for_connection_state(&self.frame_state.frame_connected);
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

/// Helper function to set the icon based on connection status
pub(crate) fn set_icon_for_connection_state(state: &ConnectionState) {
    let path = match state {
        ConnectionState::Connected => "icons/icon96good.png",
        ConnectionState::Disconnected => "icons/icon96moon.png",
    };

    let details = SetIconDetails::new();
    details.set_path(&JsValue::from_str(path));
    let _ = action::set_icon(details.into());
}
