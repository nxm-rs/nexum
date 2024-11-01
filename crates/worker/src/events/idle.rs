use std::sync::Arc;

use futures::lock::Mutex;
use tracing::debug;
use wasm_bindgen::JsValue;

use crate::Extension;

// To be used with the `chrome.idle.onStateChanged` event
pub async fn idle_on_state_changed(extension: Arc<Mutex<Extension>>, state: JsValue) {
    if state == "active" {
        debug!("Idle state changed to active");
        let mut extension = extension.lock().await;
        extension.destroy_provider().await;
        extension.init_provider().await;
    }
}
