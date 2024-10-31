use js_sys::Function;
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::prelude::*;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlarmInfo {
    pub period_in_minutes: Option<f64>,
    pub scheduled_time: f64,
    pub name: String,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AlarmCreateInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delay_in_minutes: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub period_in_minutes: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when: Option<f64>,
}

#[wasm_bindgen]
extern "C" {
    // Binding for chrome.alarms.get
    #[wasm_bindgen(js_namespace = ["chrome", "alarms"], js_name = get)]
    fn getAlarm(name: &str, callback: &Function);

    // Binding for chrome.alarms.create
    #[wasm_bindgen(js_namespace = ["chrome", "alarms"], js_name = create)]
    fn createAlarm(name: &str, alarm_info: &JsValue);

    // Binding for chrome.alarms.onAlarm.addListener
    #[wasm_bindgen(js_namespace = ["chrome", "alarms", "onAlarm"], js_name = addListener)]
    fn addAlarmListener(callback: &Function);
}

// Rust wrappers

// Function to get an alarm
pub async fn get(name: &str) -> Result<Option<AlarmInfo>, JsValue> {
    let (sender, receiver) = futures::channel::oneshot::channel();
    let callback = Closure::once_into_js(move |response: JsValue| {
        let _ = sender.send(response);
    });
    getAlarm(name, callback.unchecked_ref());

    receiver
        .await
        .map(|response| {
            if response.is_undefined() {
                None // Return None if the response is undefined
            } else {
                from_value(response)
                    .map(Some) // Return Some(AlarmInfo) if parsing succeeds
                    .map_err(|_| JsValue::from_str("Failed to parse response"))
                    .unwrap()
            }
        })
        .map_err(|_| JsValue::from_str("Failed to receive response"))
}

// Function to create an alarm
pub async fn create_alarm(name: &str, alarm_info: AlarmCreateInfo) -> Result<(), JsValue> {
    let alarm_info_js = to_value(&alarm_info).map_err(|e| JsValue::from_str(&e.to_string()))?;
    createAlarm(name, &alarm_info_js);
    Ok(())
}

// Function to add an alarm listener
pub async fn on_alarm_add_listener(callback: &Function) -> Result<(), JsValue> {
    addAlarmListener(callback);
    Ok(())
}
