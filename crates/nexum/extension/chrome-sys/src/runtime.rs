use js_sys::{Function, Promise};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    // Binding for chrome.runtime.connect
    #[wasm_bindgen(js_namespace = ["chrome", "runtime"], js_name = connect, catch)]
    pub fn connect(connect_info: &JsValue) -> Result<JsValue, JsValue>;

    // Binding for chrome.runtime.sendMessage
    #[wasm_bindgen(js_namespace = ["chrome", "runtime"], js_name = sendMessage, catch)]
    pub fn send_message(message: &JsValue) -> Result<Promise, JsValue>;

    // Binding for chrome.runtime.onMessage.addListener
    #[wasm_bindgen(js_namespace = ["chrome", "runtime", "onMessage"], js_name = addListener)]
    pub fn add_on_message_listener(callback: &Function);

    // Binding for chrome.runtime.onConnect.addListener
    #[wasm_bindgen(js_namespace = ["chrome", "runtime", "onConnect"], js_name = addListener)]
    pub fn add_on_connect_listener(callback: &Function);

    // Binding for chrome.runtime.getURL
    #[wasm_bindgen(js_namespace = ["chrome", "runtime"], js_name = getURL)]
    pub fn getURL(path: &str) -> String;
}
