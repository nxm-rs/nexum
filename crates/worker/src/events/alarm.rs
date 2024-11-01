use std::sync::Arc;

use chrome_sys::alarms;
use jsonrpsee::{core::client::ClientT, wasm_client::Client};
use serde_wasm_bindgen::from_value;
use tracing::{error, info};
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;

use crate::CLIENT_STATUS_ALARM_KEY;

// To be used with the `chrome.alarms.onAlarm` event
pub fn on_alarm(provider: Arc<Client>, alarm: JsValue) {
    let alarm: alarms::AlarmInfo = from_value(alarm).unwrap();

    if alarm.name == CLIENT_STATUS_ALARM_KEY {
        // Here we reguarly check the RPC client status by requesting `web3_clientVersion`
        // If the client is not connected, we should try to reconnect

        if provider.is_connected() {
            // Make the `web3_clientVersion` RPC call
            spawn_local(async move {
                match provider.request::<String, _>("web3_clientVersion", Vec::<String>::new()).await {
                    Ok(result) => {
                        info!("alarm keepalive web3_clientVersion result: {}", result);
                    }
                    Err(e) => {
                        error!("alarm RPC call failed: {:?}", e);
                    }
                }
            });
        }
    }
}
