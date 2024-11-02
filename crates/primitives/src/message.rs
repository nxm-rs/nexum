use derive_more::Display;
use gloo_utils::format::JsValueSerdeExt;
use serde::de::{self};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_wasm_bindgen::from_value;
use tracing::trace;
use wasm_bindgen::prelude::*;

// Define the common base structure with optional fields
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MessagePayloadBase {
    jsonrpc: String,
    id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    origin: Option<String>,
}

// EthEventPayload with Ethereum-specific event details
#[derive(Serialize, Deserialize, Debug, Clone, Display)]
#[display("EthEvent {{ event: {}, args: {:?} }}", event, args)]
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
#[derive(Serialize, Deserialize, Debug, Clone, Display)]
#[display(
    "EthPayload {{ jsonrpc: {}, id: {}, method: {:?}, params: {:?}, result: {:?} }}",
    base.jsonrpc, base.id, method, params, result
)]
#[serde(rename_all = "camelCase")]
pub struct EthPayload {
    #[serde(flatten)]
    pub base: MessagePayloadBase,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<serde_json::Value>,
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
                origin: None,
            },
            method,
            params,
            result,
            error: None,
        }
    }
}

// EmbeddedActionPayload with flexible action details
#[derive(Serialize, Deserialize, Debug, Clone, Display)]
#[display("EmbeddedActionPayload {{ action: {} }}", action)]
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
#[derive(Serialize, Deserialize, Debug, Clone, Display)]
#[display("EmbeddedAction {{ action_type: {}, data: {:?} }}", action_type, data)]
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
#[derive(Serialize, Deserialize, Debug, Clone, Display)]
#[display("ChainChangedPayload {{ chain_id: {} }}", chain_id)]
#[serde(rename_all = "camelCase")]
pub struct ChainChangedPayload {
    chain_id: u64,
}

// Enum to wrap each variant and set the "type" field automatically
#[derive(Debug, Clone, Display)]
pub enum MessagePayload {
    EthEvent(EthEventPayload),
    JsonResponse(EthPayload),
    EmbeddedAction(EmbeddedActionPayload),
    ChainChanged(ChainChangedPayload),
    JsonRequest(EthPayload),
}

// Custom serializer for MessagePayload
impl Serialize for MessagePayload {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Helper function to serialize data with a type field
        fn serialize_with_type<S>(
            data: &impl Serialize,
            type_str: &str,
            serializer: S,
        ) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let mut map = serde_json::to_value(data).unwrap();
            map.as_object_mut().unwrap().insert(
                "type".to_string(),
                serde_json::Value::String(type_str.to_string()),
            );
            map.serialize(serializer)
        }

        // Match arms now only specify the payload data and type string
        match self {
            MessagePayload::EthEvent(data) => serialize_with_type(data, "eth:event", serializer),
            MessagePayload::JsonResponse(data) => {
                serialize_with_type(data, "eth:payload", serializer)
            }
            MessagePayload::EmbeddedAction(data) => {
                serialize_with_type(data, "embedded:action", serializer)
            }
            MessagePayload::ChainChanged(data) => {
                serialize_with_type(data, "chainChanged", serializer)
            }
            MessagePayload::JsonRequest(data) => data.serialize(serializer),
        }
    }
}

// Custom deserializer for MessagePayload
impl<'de> Deserialize<'de> for MessagePayload {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let map = serde_json::Value::deserialize(deserializer)?;

        match map.get("type").and_then(|t| t.as_str()) {
            Some("eth:event") => serde_json::from_value(map)
                .map(MessagePayload::EthEvent)
                .map_err(de::Error::custom),
            Some("eth:payload") => serde_json::from_value(map)
                .map(MessagePayload::JsonResponse)
                .map_err(de::Error::custom),
            Some("embedded:action") => serde_json::from_value(map)
                .map(MessagePayload::EmbeddedAction)
                .map_err(de::Error::custom),
            Some("chainChanged") => serde_json::from_value(map)
                .map(MessagePayload::ChainChanged)
                .map_err(de::Error::custom),
            None => serde_json::from_value(map)
                .map(MessagePayload::JsonRequest)
                .map_err(de::Error::custom),
            Some(unknown) => Err(de::Error::unknown_variant(
                unknown,
                &[
                    "eth:event",
                    "eth:payload",
                    "embedded:action",
                    "chainChanged",
                ],
            )),
        }
    }
}

// Conversion functions for JsValue compatibility
impl MessagePayload {
    // Convert MessagePayload to JsValue for passing to other Rust functions
    pub fn to_js_value(&self) -> JsValue {
        JsValue::from_serde(self).expect("Failed to serialize MessagePayload to JsValue")
    }

    // Convert JsValue back to MessagePayload
    pub fn from_js_value(js_value: &JsValue) -> Result<Self, JsValue> {
        js_value.into_serde().map_err(|_| {
            trace!(
                "Failed to deserialize JsValue into MessagePayload: {:?}",
                js_value
            );
            JsValue::from_str("Failed to deserialize JsValue into MessagePayload")
        })
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
        let payload = MessagePayload::EthEvent(EthEventPayload {
            event: "accountsChanged".to_string(),
            args: vec![
                json!({"account": "0x12345", "balance": 1000}),
                json!({"account": "0x67890", "balance": 2000}),
            ],
        });

        let js_value = payload.to_js_value();
        console::log_1(&"Serialized payload to JsValue".into());
        console::log_1(&js_value);

        let deserialized_payload: MessagePayload =
            MessagePayload::from_js_value(&js_value).expect("Deserialization failed");

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

    #[wasm_bindgen_test]
    fn test_deserialization_bare_request() {
        let js_value = JsValue::from_serde(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eth_chainId",
            "params": []
        }))
        .expect("Failed to convert JSON to JsValue");

        let deserialized_payload: MessagePayload =
            MessagePayload::from_js_value(&js_value).expect("Deserialization failed");

        console::log_1(&"Deserialized JsValue into MessagePayload".into());
        console::log_1(&JsValue::from_serde(&deserialized_payload).unwrap());

        match deserialized_payload {
            MessagePayload::JsonRequest(bare_request) => {
                console::log_1(&"Deserialized BareRequest matches expected".into());
                assert_eq!(bare_request.base.jsonrpc, "2.0");
                assert_eq!(bare_request.base.id, 1);
                assert_eq!(bare_request.method, Some("eth_chainId".to_string()));
                assert_eq!(bare_request.params, Some(vec![]));
            }
            _ => panic!("Deserialized payload type does not match BareRequest"),
        }
    }
}
