use js_sys::{Function, Promise};
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::from_value;
use tracing::error;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlarmInfo {
    pub period_in_minutes: Option<f64>,
    pub scheduled_time: f64,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
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
    fn get_alarms_js(name: &str) -> Promise;

    // Binding for chrome.alarms.create
    #[wasm_bindgen(js_namespace = ["chrome", "alarms"], js_name = create)]
    pub fn create(name: &str, alarm_info: &JsValue);

    // Binding for chrome.alarms.onAlarm.addListener
    #[wasm_bindgen(js_namespace = ["chrome", "alarms", "onAlarm"], js_name = addListener)]
    pub fn add_alarm_listener(callback: &Function);
}

// Rust wrappers
pub async fn get(name: &str) -> Result<Option<AlarmInfo>, JsValue> {
    let promise = get_alarms_js(name);
    let result = JsFuture::from(promise).await?;

    if result.is_undefined() {
        Ok(None)
    } else {
        match from_value(result) {
            Ok(alarm_info) => {
                Ok(Some(alarm_info))
            }
            Err(err) => {
                error!("Failed to parse response for alarm '{}': {:?}", name, err);
                Err(JsValue::from_str(&format!(
                    "Failed to parse response: {:?}",
                    err
                )))
            }
        }
    }
}
