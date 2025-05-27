use std::sync::Arc;

use tracing::debug;
use wasm_bindgen::JsValue;

use crate::Extension;

// To be used with the `chrome.idle.onStateChanged` event
pub async fn idle_on_state_changed(extension: Arc<Extension>, state: JsValue) {
    if state == "active" {
        debug!("Idle state changed to active");

        // Check if the provider exists and call reset
        if let Some(provider) = &extension.provider {
            provider.reset().await;
        } else {
            debug!("Provider is not initialized.");
        }
    }
}
