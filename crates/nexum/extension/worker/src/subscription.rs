use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::warn;
use wasm_bindgen::JsValue;

#[derive(Serialize, Deserialize)]
pub(crate) enum SubscriptionType {
    ChainChanged,
    ChainsChanged,
    AccountsChanged,
    NetworkChanged,
    Message,
    Unknown,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Subscription {
    pub tab_id: u32,
    pub type_: SubscriptionType,
}

pub(crate) async fn unsubscribe(sub_id: String) -> Result<(), JsValue> {
    warn!("TODO: Unsubscribing from {:?}", sub_id);
    // Unsubscribe logic here

    Ok(())
}

pub(crate) fn sub_type(params: &JsValue) -> SubscriptionType {
    match serde_wasm_bindgen::from_value::<Value>(params.clone()) {
        Ok(Value::Array(params_vec)) => {
            if let Some(Value::String(sub_type_str)) = params_vec.first() {
                match sub_type_str.as_str() {
                    "ChainChanged" => SubscriptionType::ChainChanged,
                    "ChainsChanged" => SubscriptionType::ChainsChanged,
                    "AccountsChanged" => SubscriptionType::AccountsChanged,
                    "NetworkChanged" => SubscriptionType::NetworkChanged,
                    "Message" => SubscriptionType::Message,
                    _ => SubscriptionType::Unknown,
                }
            } else {
                warn!("First parameter is not a string");
                SubscriptionType::Unknown
            }
        }
        Ok(_) => {
            warn!("Params is not an array");
            SubscriptionType::Unknown
        }
        Err(_) => {
            warn!("Error parsing params as JSON array");
            SubscriptionType::Unknown
        }
    }
}
