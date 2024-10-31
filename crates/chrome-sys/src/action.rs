use std::collections::HashMap;

use serde::Serialize;
use serde_json::Value;
use serde_wasm_bindgen::to_value;
use wasm_bindgen::prelude::*;

// Define the types for path and imageData as an enum to handle single or dictionary values
#[derive(Serialize)]
#[serde(untagged, rename_all = "camelCase")]
pub enum IconPath {
    Single(String),
    Dictionary(HashMap<u32, String>),
}

#[derive(Serialize)]
#[serde(untagged, rename_all = "camelCase")]
pub enum IconImageData {
    Single(Value), // JsValue is used to represent ImageData in WebAssembly
    Dictionary(HashMap<u32, Value>),
}

// Define the main TabIconDetails struct
#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TabIconDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<IconPath>, // Optional field for path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tab_id: Option<u32>, // Optional field for tabId
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_data: Option<IconImageData>, // Optional field for imageData
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PopupDetails {
    pub popup: String, // The HTML file to show in the popup
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tab_id: Option<u32>, // Optional field for tab ID
}

#[wasm_bindgen]
extern "C" {
    // Binding for chrome.action.setIcon
    #[wasm_bindgen(js_namespace = ["chrome", "action"], js_name = setIcon)]
    fn setIcon(details: &JsValue);

    // Binding for chrome.action.setPopup
    #[wasm_bindgen(js_namespace = ["chrome", "action"], js_name = setPopup)]
    fn setPopup(details: &JsValue);
}

// Rust wrappers

// Wrapper for set_icon (no async needed as there's no return)
pub fn set_icon(details: TabIconDetails) -> Result<(), JsValue> {
    // Convert TabIconDetails to JsValue
    let details_js = to_value(&details).map_err(|e| JsValue::from_str(&e.to_string()))?;
    setIcon(&details_js);
    Ok(())
}

// Wrapper for set_popup (no async needed as there's no return)
pub fn set_popup(details: PopupDetails) -> Result<(), JsValue> {
    // Convert PopupDetails to JsValue
    let details_js = to_value(&details).map_err(|e| JsValue::from_str(&e.to_string()))?;
    setPopup(&details_js);
    Ok(())
}
