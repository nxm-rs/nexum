use chrome_sys::alarms;
use jsonrpsee::core::client::ClientT;
use serde_wasm_bindgen::from_value;
use tracing::{error, info};
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;

use crate::{provider::ProviderType, CLIENT_STATUS_ALARM_KEY};

// To be used with the `chrome.alarms.onAlarm` event
pub async fn on_alarm(provider: ProviderType, alarm: JsValue) {
    let alarm: alarms::AlarmInfo = from_value(alarm).unwrap();

    if alarm.name == CLIENT_STATUS_ALARM_KEY {
        let provider_clone = provider.clone(); // Clone the Arc for use in spawn_local
        spawn_local(async move {
            match provider_clone.read() {
                Ok(provider_guard) => {
                    // Handle the read lock Result
                    if let Some(client) = provider_guard.as_ref() {
                        if client.is_connected() {
                            match client
                                .request::<String, _>("web3_clientVersion", Vec::<String>::new())
                                .await
                            {
                                Ok(result) => {
                                    info!("alarm keepalive web3_clientVersion result: {}", result);
                                }
                                Err(e) => {
                                    error!("alarm RPC call failed: {:?}", e);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to acquire read lock on provider: {:?}", e);
                }
            }
        });
    }
}
