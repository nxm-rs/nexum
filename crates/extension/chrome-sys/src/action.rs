use std::collections::HashMap;

use serde::Serialize;
use serde_json::Value;
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
    Single(Value),
    Dictionary(HashMap<u32, Value>),
}

// Define the main TabIconDetails struct
#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TabIconDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<IconPath>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tab_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_data: Option<IconImageData>,
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PopupDetails {
    pub popup: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tab_id: Option<u32>,
}

#[wasm_bindgen]
extern "C" {
    // Binding for chrome.action.setIcon
    #[wasm_bindgen(js_namespace = ["chrome", "action"], js_name = setIcon)]
    pub fn set_icon(details: JsValue);

    // Binding for chrome.action.setPopup
    #[wasm_bindgen(js_namespace = ["chrome", "action"], js_name = setPopup, catch)]
    pub fn set_popup(details: JsValue) -> Result<(), JsValue>;
}
