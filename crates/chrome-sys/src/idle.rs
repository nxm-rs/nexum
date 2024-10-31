use js_sys::Function;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    // Binding for chrome.idle.onStateChanged.addListener
    #[wasm_bindgen(js_namespace = ["chrome", "idle", "onStateChanged"], js_name = addListener)]
    fn idleAddOnStateChangedListener(callback: &Function);
}

// Rust wrappers

// Wrapper for idleAddOnStateChangedListener
pub fn on_state_changed_add_listener(callback: &Function) -> Result<(), JsValue> {
    idleAddOnStateChangedListener(callback);
    Ok(())
}
