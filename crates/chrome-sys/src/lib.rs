use std::collections::HashMap;

use js_sys::{Function, Promise};
use serde::Serialize;
use serde_json::Value;
use serde_wasm_bindgen::to_value;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

#[derive(Serialize, Default)]
pub struct QueryInfo {
    pub status: Option<String>, // "loading" | "complete"
    pub last_focused_window: Option<bool>,
    pub window_id: Option<i32>, // i32 for window ID or windows.WINDOW_ID_CURRENT
    pub window_type: Option<String>, // "normal" | "popup" | "panel" | "app" | "devtools"
    pub active: Option<bool>,
    pub index: Option<i32>,
    pub title: Option<String>,
    pub url: Option<Value>, // String or array of strings
    pub current_window: Option<bool>,
    pub highlighted: Option<bool>,
    pub discarded: Option<bool>,
    pub auto_discardable: Option<bool>,
    pub pinned: Option<bool>,
    pub audible: Option<bool>,
    pub muted: Option<bool>,
    pub group_id: Option<i32>,
}

// Define the types for path and imageData as an enum to handle single or dictionary values
#[derive(Serialize)]
#[serde(untagged)]
pub enum IconPath {
    Single(String),
    Dictionary(HashMap<u32, String>),
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum IconImageData {
    Single(Value), // JsValue is used to represent ImageData in WebAssembly
    Dictionary(HashMap<u32, Value>),
}

// Define the main TabIconDetails struct
#[derive(Serialize, Default)]
pub struct TabIconDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<IconPath>, // Optional field for path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tab_id: Option<u32>, // Optional field for tabId
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_data: Option<IconImageData>, // Optional field for imageData
}

#[derive(Serialize, Default)]
pub struct PopupDetails {
    pub popup: String, // The HTML file to show in the popup
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tab_id: Option<u32>, // Optional field for tab ID
}

#[wasm_bindgen(module = "/src/chrome_bindings.js")]
extern "C" {
    // Binding for setIcon (no return value)
    fn setIcon(details: &JsValue);

    // Binding for setPopup (no return value)
    fn setPopup(details: &JsValue);

    // Binding for sendMessageToTab (returns Promise)
    #[wasm_bindgen(catch)]
    fn sendMessageToTab(tab_id: u32, message: &JsValue) -> Result<Promise, JsValue>;

    // Binding for queryTabs (returns Promise)
    #[wasm_bindgen(catch)]
    fn queryTabs(query_info: &JsValue) -> Result<Promise, JsValue>;

    #[wasm_bindgen(catch)]
    fn getTab(tab_id: JsValue) -> Result<Promise, JsValue>;

    #[wasm_bindgen(catch)]
    fn getAlarm(name: &str) -> Result<Promise, JsValue>;

    #[wasm_bindgen(catch)]
    fn portAddOnDisconnectListener(port: &JsValue, callback: &Function) -> Result<(), JsValue>;

    #[wasm_bindgen(catch)]
    fn portRemoveOnDisconnectListener(port: &JsValue, callback: &Function) -> Result<(), JsValue>;

    #[wasm_bindgen(catch)]
    fn portPostMessage(port: &JsValue, message: &JsValue) -> Result<(), JsValue>;

    #[wasm_bindgen(catch)]
    fn addTabRemovedListener(callback: &Function) -> Result<(), JsValue>;

    #[wasm_bindgen(catch)]
    fn addTabUpdatedListener(callback: &Function) -> Result<(), JsValue>;

    #[wasm_bindgen(catch)]
    fn addTabActivatedListener(callback: &Function) -> Result<(), JsValue>;

    #[wasm_bindgen(catch)]
    fn createAlarm(name: &str, alarmInfo: &JsValue) -> Result<(), JsValue>;

    #[wasm_bindgen(catch)]
    fn addAlarmListener(callback: &Function) -> Result<(), JsValue>;
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

// Wrapper for send_message_to_tab, directly taking a JsValue message
pub async fn send_message_to_tab(tab_id: u32, message: JsValue) -> Result<JsValue, JsValue> {
    let promise = sendMessageToTab(tab_id, &message)?;
    JsFuture::from(promise).await
}

// Function to query tabs with QueryInfo struct
pub async fn query_tabs(query_info: QueryInfo) -> Result<JsValue, JsValue> {
    // Serialize QueryInfo to JsValue
    let query_info_js = to_value(&query_info).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let promise = queryTabs(&query_info_js)?; // Pass the serialized object to JS
    JsFuture::from(promise).await
}

pub async fn get_tab(tab_id: u32) -> Result<JsValue, JsValue> {
    let tab_id_js = JsValue::from(tab_id);
    let promise = getTab(tab_id_js)?;
    JsFuture::from(promise).await
}

// Function to get an alarm
pub async fn get_alarm(name: &str) -> Result<JsValue, JsValue> {
    let promise = getAlarm(name)?;
    JsFuture::from(promise).await
}

// Function to create an alarm
pub async fn create_alarm(name: &str, alarm_info: JsValue) -> Result<(), JsValue> {
    createAlarm(name, &alarm_info)
}

// Function to add an alarm listener
pub async fn add_alarm_listener(callback: &Function) -> Result<(), JsValue> {
    addAlarmListener(callback)
}

// Updated wrapper function that accepts a pre-made `&Function` reference
pub fn port_add_on_disconnect_listener(port: JsValue, callback: &Function) -> Result<(), JsValue> {
    portAddOnDisconnectListener(&port, callback)
}

pub fn port_remove_on_disconnect_listener(
    port: JsValue,
    callback: &Function,
) -> Result<(), JsValue> {
    portRemoveOnDisconnectListener(&port, callback)
}

// Wrapper for portPostMessage
pub fn port_post_message(port: JsValue, message: JsValue) -> Result<(), JsValue> {
    portPostMessage(&port, &message)
}

// Wrapper for addTabRemovedListener
pub fn add_tab_removed_listener(callback: &Function) -> Result<(), JsValue> {
    addTabRemovedListener(callback)
}

// Wrapper for addTabUpdatedListener
pub fn add_tab_updated_listener(callback: &Function) -> Result<(), JsValue> {
    addTabUpdatedListener(callback)
}

// Wrapper for addTabActivatedListener
pub fn add_tab_activated_listener(callback: &Function) -> Result<(), JsValue> {
    addTabActivatedListener(callback)
}
