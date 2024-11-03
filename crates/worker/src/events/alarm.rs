use chrome_sys::alarms;
use serde_wasm_bindgen::from_value;
use tracing::{error, info};
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;

use crate::{Extension, CLIENT_STATUS_ALARM_KEY};
use std::sync::Arc;

// To be used with the `chrome.alarms.onAlarm` event
pub async fn on_alarm(extension: Arc<Extension>, alarm: JsValue) {
    // Attempt to deserialize the alarm info and log an error if it fails
    let alarm: alarms::AlarmInfo = match from_value(alarm) {
        Ok(alarm) => alarm,
        Err(e) => {
            error!("Failed to parse alarm info: {:?}", e);
            return;
        }
    };

    if alarm.name == CLIENT_STATUS_ALARM_KEY {
        // Retrieve the provider from the extension
        if let Some(provider) = &extension.provider {
            let provider_clone = provider.clone();
            spawn_local(async move {
                // Use Provider's methods to handle connection checks
                if provider_clone.is_connected().await {
                    match provider_clone
                        .request::<String>("web3_clientVersion", Vec::<String>::new())
                        .await
                    {
                        Ok(result) => {
                            info!("alarm keepalive web3_clientVersion result: {}", result);
                        }
                        Err(e) => {
                            error!("alarm RPC call failed: {:?}", e);
                        }
                    }
                } else {
                    error!("Provider is not connected");
                }
            });
        } else {
            error!("Provider is not initialized in the extension");
        }
    }
}
