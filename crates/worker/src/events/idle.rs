use std::sync::Arc;

use tracing::debug;
use wasm_bindgen::JsValue;

use crate::{
    provider::{destroy_provider, init_provider},
    Extension,
};

// To be used with the `chrome.idle.onStateChanged` event
pub async fn idle_on_state_changed(extension: Arc<Extension>, state: JsValue) {
    if state == "active" {
        debug!("Idle state changed to active");
        let extension_clone = extension.clone();
        destroy_provider(extension_clone).await;
        init_provider(extension).await;
    }
}
