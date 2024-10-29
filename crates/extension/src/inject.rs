use gloo_utils::format::JsValueSerdeExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::warn;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use web_sys::window;

// Define the InjectedPayload as described
#[derive(Serialize, Deserialize)]
pub struct InjectedPayload {
    #[serde(rename = "type")]
    pub payload_type: String,
    pub data: Value,
}

// Main setup function
#[wasm_bindgen]
pub fn setup_message_handlers() {
    // Closure to handle messages from `chrome.runtime`
    let message_handler = Closure::wrap(Box::new(move |payload: JsValue| {
        // Use `into_serde` to try deserializing the `JsValue` payload into `InjectedPayload`
        if let Ok(incoming_message) = payload.into_serde::<InjectedPayload>() {
            match incoming_message.payload_type.as_str() {
                "eth:payload" | "embedded:action" | "eth:event" => {
                    if let Err(e) = forward_to_page(incoming_message) {
                        warn!("{}", &format!("Failed to forward message to page: {:?}", e));
                    }
                }
                _ => warn!("Received unknown message type."),
            }
        } else {
            warn!("Failed to deserialize payload.");
        }
    }) as Box<dyn FnMut(JsValue)>);

    // Register the message handler
    let global = js_sys::global();
    let mixin: ExtensionGlobalsMixin = global.unchecked_into();

    mixin.chrome().runtime().on_message_external().add_listener(
        message_handler.as_ref().unchecked_ref(),
        JsValue::undefined(),
    );
    message_handler.forget(); // Avoid memory leak
}

// Function to send a message from Rust back to the Chrome extension's background script
#[wasm_bindgen]
pub fn send_message_to_background(payload_type: &str, data: JsValue) -> Result<(), JsValue> {
    let payload = js_sys::Object::new();
    js_sys::Reflect::set(
        &payload,
        &JsValue::from("type"),
        &JsValue::from(payload_type),
    )?;
    js_sys::Reflect::set(&payload, &JsValue::from("data"), &data)?;

    let global = js_sys::global();
    let mixin: ExtensionGlobalsMixin = global.unchecked_into();
    mixin.chrome().runtime().send_message(&payload);
    Ok(())
}

// Forward the message to the page (e.g., from the background to the content script)
fn forward_to_page(payload: InjectedPayload) -> Result<(), JsValue> {
    let message =
        JsValue::from_serde(&payload).map_err(|e| JsValue::from_str(&format!("{}", e)))?;
    post_message_to_window(&message)
}

// Function to post messages using `web-sys`
fn post_message_to_window(data: &JsValue) -> Result<(), JsValue> {
    let window = window().ok_or_else(|| JsValue::from_str("no global `window` exists"))?;
    let origin = window
        .location()
        .origin()
        .map_err(|_| JsValue::from_str("could not get origin"))?;
    window.post_message(data, &origin)
}
