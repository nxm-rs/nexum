use js_sys::{Function, Promise};
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::{from_value, to_value};
use tracing::{error, info, warn};
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
    fn getAlarm(name: &str) -> Promise;

    // Binding for chrome.alarms.create
    #[wasm_bindgen(js_namespace = ["chrome", "alarms"], js_name = create)]
    fn createAlarm(name: &str, alarm_info: &JsValue);

    // Binding for chrome.alarms.onAlarm.addListener
    #[wasm_bindgen(js_namespace = ["chrome", "alarms", "onAlarm"], js_name = addListener)]
    fn addAlarmListener(callback: &Function);
}

// Rust wrappers

// Rust wrapper function for `getAlarm`
pub async fn get(name: &str) -> Result<Option<AlarmInfo>, JsValue> {
    let promise = getAlarm(name);
    let result = JsFuture::from(promise).await;

    match result {
        Ok(response) => {
            // If the response is undefined, return None
            if response.is_undefined() {
                info!("No alarm found with name: {}", name);
                Ok(None)
            } else {
                // Attempt to parse the response as `AlarmInfo`
                match from_value(response) {
                    Ok(alarm_info) => {
                        info!("Successfully retrieved alarm: {:?}", alarm_info);
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
        Err(err) => {
            warn!("Failed to retrieve alarm '{}': {:?}", name, err);
            Err(JsValue::from_str("Failed to retrieve alarm"))
        }
    }
}

// Function to create an alarm
pub async fn create_alarm(name: &str, alarm_info: AlarmCreateInfo) -> Result<(), JsValue> {
    let alarm_info_js = to_value(&alarm_info).map_err(|e| JsValue::from_str(&e.to_string()))?;
    createAlarm(name, &alarm_info_js);
    Ok(())
}

// Function to add an alarm listener
pub fn on_alarm_add_listener(callback: &Function) -> Result<(), JsValue> {
    addAlarmListener(callback);
    Ok(())
}
