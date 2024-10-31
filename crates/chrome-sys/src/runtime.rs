use js_sys::Function;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    // Binding for chrome.runtime.connect
    #[wasm_bindgen(js_namespace = ["chrome", "runtime"], js_name = connect)]
    fn runtimeConnect(connect_info: &JsValue) -> JsValue;

    // Binding for chrome.runtime.sendMessage
    #[wasm_bindgen(js_namespace = ["chrome", "runtime"], js_name = sendMessage)]
    fn runtimeSendMessage(message: &JsValue, callback: &Function);

    // Binding for chrome.runtime.onMessage.addListener
    #[wasm_bindgen(js_namespace = ["chrome", "runtime", "onMessage"], js_name = addListener)]
    fn runtimeAddOnMessageListener(callback: &Function);

    // Binding for chrome.runtime.onConnect.addListener
    #[wasm_bindgen(js_namespace = ["chrome", "runtime", "onConnect"], js_name = addListener)]
    fn runtimeAddOnConnectListener(callback: &Function);
}

// Rust wrappers

// Wrapper for runtimeConnect
pub fn runtime_connect(connect_info: JsValue) -> Result<JsValue, JsValue> {
    Ok(runtimeConnect(&connect_info))
}

// Wrapper for runtimeSendMessage
pub async fn runtime_send_message(message: JsValue) -> Result<JsValue, JsValue> {
    let (sender, receiver) = futures::channel::oneshot::channel();
    let callback = Closure::once_into_js(move |response: JsValue| {
        let _ = sender.send(response);
    });
    runtimeSendMessage(&message, callback.unchecked_ref());
    receiver
        .await
        .map_err(|_| JsValue::from_str("Failed to receive response"))
}

// Wrapper for runtimeAddOnMessageListener
pub fn on_message_add_listener(callback: &Function) -> Result<(), JsValue> {
    runtimeAddOnMessageListener(callback);
    Ok(())
}

// Wrapper for runtimeAddOnConnectListener
pub fn on_connect_add_listener(callback: &Function) -> Result<(), JsValue> {
    runtimeAddOnConnectListener(callback);
    Ok(())
}
