use gloo_utils::format::JsValueSerdeExt;
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::from_value;
use wasm_bindgen::prelude::*;

// Define the common base structure with optional fields
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct MessagePayloadBase {
    jsonrpc: String,
    id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    frame_origin: Option<String>, // '__frameOrigin' in JS
}

// EthEventPayload with Ethereum-specific event details
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EthEventPayload {
    event: String,
    args: Vec<serde_json::Value>, // Allow varied argument types
}

impl EthEventPayload {
    // Create a new EthEventPayload with the given event name and arguments
    pub fn new(event: String, args: JsValue) -> Self {
        let args: Vec<serde_json::Value> = from_value(args).unwrap_or(vec![]);
        EthEventPayload { event, args }
    }
}

// EthPayload for JSON-RPC responses
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EthPayload {
    #[serde(flatten)]
    base: MessagePayloadBase,
    #[serde(skip_serializing_if = "Option::is_none")]
    method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
}

impl EthPayload {
    // Create a new EthPayload with the given ID and optional method, params, and result
    pub fn new(
        id: u64,
        method: Option<String>,
        params: Option<Vec<serde_json::Value>>,
        result: Option<serde_json::Value>,
    ) -> Self {
        EthPayload {
            base: MessagePayloadBase {
                jsonrpc: "2.0".to_string(),
                id,
                frame_origin: None,
            },
            method,
            params,
            result,
        }
    }
}

// EmbeddedActionPayload with flexible action details
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EmbeddedActionPayload {
    action: EmbeddedAction,
}

impl EmbeddedActionPayload {
    // Create a new EmbeddedActionPayload with the given action
    pub fn new(action: EmbeddedAction) -> Self {
        EmbeddedActionPayload { action }
    }
}

// Action structure to hold action type and additional fields
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EmbeddedAction {
    action_type: String, // 'type' in JS
    #[serde(flatten)]
    data: serde_json::Value, // Additional fields
}

impl EmbeddedAction {
    // Create a new EmbeddedAction with the given type and data
    pub fn new(action_type: String, data: JsValue) -> Self {
        EmbeddedAction {
            action_type,
            data: from_value(data).unwrap_or(serde_json::Value::Null),
        }
    }
}

// ChainChangedPayload with chain ID specifics
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChainChangedPayload {
    chain_id: u64,
}

// Enum to wrap each variant and set the "type" field automatically
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum MessagePayload {
    #[serde(rename = "eth:event")]
    EthEvent(EthEventPayload),
    #[serde(rename = "eth:payload")]
    Eth(EthPayload),
    #[serde(rename = "embedded:action")]
    EmbeddedAction(EmbeddedActionPayload),
    #[serde(rename = "chainChanged")]
    ChainChanged(ChainChangedPayload),
}

// Conversion functions for JsValue compatibility
impl MessagePayload {
    // Convert MessagePayload to JsValue for passing to other Rust functions
    pub fn to_js_value(&self) -> JsValue {
        JsValue::from_serde(self).expect("Failed to serialize MessagePayload to JsValue")
    }

    // Convert JsValue back to MessagePayload
    pub fn from_js_value(js_value: &JsValue) -> Result<Self, JsValue> {
        js_value
            .into_serde()
            .map_err(|_| JsValue::from_str("Failed to deserialize JsValue into MessagePayload"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wasm_bindgen_test::wasm_bindgen_test;
    use web_sys::console;

    #[wasm_bindgen_test]
    fn test_serialization_deserialization_eth_event() {
        // Create a complex EthEventPayload
        let payload = MessagePayload::EthEvent(EthEventPayload {
            event: "accountsChanged".to_string(),
            args: vec![
                json!({"account": "0x12345", "balance": 1000}),
                json!({"account": "0x67890", "balance": 2000}),
            ],
        });

        // Serialize to JsValue
        let js_value = payload.to_js_value();
        console::log_1(&"Serialized payload to JsValue".into());
        console::log_1(&js_value);

        // Deserialize back to MessagePayload
        let deserialized_payload: MessagePayload =
            MessagePayload::from_js_value(&js_value).expect("Deserialization failed");

        // Verify that the original and deserialized payloads match
        match deserialized_payload {
            MessagePayload::EthEvent(deserialized_event) => {
                console::log_1(&"Deserialized payload matches expected".into());
                assert_eq!(deserialized_event.event, "accountsChanged");
                assert_eq!(deserialized_event.args.len(), 2);
                assert_eq!(deserialized_event.args[0]["account"], "0x12345");
                assert_eq!(deserialized_event.args[0]["balance"], 1000);
                assert_eq!(deserialized_event.args[1]["account"], "0x67890");
                assert_eq!(deserialized_event.args[1]["balance"], 2000);
            }
            _ => panic!("Deserialized payload type does not match"),
        }
    }
}
