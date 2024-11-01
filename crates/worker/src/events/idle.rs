use std::{cell::RefCell, rc::Rc};

use tracing::debug;
use wasm_bindgen::JsValue;

use crate::Extension;

// To be used with the `chrome.idle.onStateChanged` event
pub async fn idle_on_state_changed(extension: Rc<RefCell<Extension>>, state: JsValue) {
    if state == "active" {
        debug!("Idle state changed to active");
        let mut extension = extension.borrow_mut();
        extension.destroy_provider().await;
        extension.init_provider().await;
    }
}
