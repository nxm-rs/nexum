use derive_more::Display;
use gloo_utils::format::JsValueSerdeExt;
use js_sys::Reflect;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MessageType {
    EthEvent(EthEvent),
    Request(RequestWithId),
    Response(ResponseWithId),
}

#[derive(Serialize, Deserialize, Debug, Clone, Display)]
#[display("EthEvent {{ event: {}, args: {:?} }}", event, args)]
pub struct EthEvent {
    pub event: String,
    pub args: Vec<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Display)]
#[display("Request {{ method: {}, params: {:?} }}", method, params)]
pub struct Request {
    pub method: String,
    pub params: Option<Vec<serde_json::Value>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Display)]
#[display("Error {{ code: {}, message: {} }}", code, message)]
pub struct Error {
    pub code: i32,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Display)]
#[display("RequestWithId {{ id: {}, request: {} }}", id, request)]
pub struct RequestWithId {
    pub id: String,
    #[serde(flatten)]
    pub request: Request,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ResponseWithId {
    pub id: String,
    #[serde(flatten)]
    pub result: Result<serde_json::Value, Error>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProtocolMessage {
    protocol: String,
    pub message: MessageType,
}

impl ProtocolMessage {
    pub fn new(message: MessageType) -> Self {
        ProtocolMessage {
            protocol: "nexum".to_string(),
            message,
        }
    }

    pub fn to_js_value(&self) -> Result<JsValue, JsValue> {
        JsValue::from_serde(self)
            .map_err(|_| JsValue::from_str("Failed to serialize ProtocolMessage to JsValue"))
    }

    // Deserialize from JsValue to ProtocolMessage
    pub fn from_js_value(js_value: &JsValue) -> Result<Self, JsValue> {
        js_value
            .into_serde()
            .map_err(|_| JsValue::from_str("Failed to deserialize JsValue into ProtocolMessage"))
    }

    pub fn is_valid(js_value: &JsValue) -> bool {
        // Quick check: must be an object with protocol="nexum" before attempting deserialize
        if !js_value.is_object() {
            return false;
        }
        let Ok(protocol) = Reflect::get(js_value, &JsValue::from_str("protocol")) else {
            return false;
        };
        if protocol.as_string().as_deref() != Some("nexum") {
            return false;
        }
        // Now safe to attempt full deserialization
        js_value.into_serde::<ProtocolMessage>().is_ok()
    }
}

// Implement the `From` trait for converting a JsValue to a ProtocolMessage
impl From<JsValue> for ProtocolMessage {
    fn from(js_value: JsValue) -> Self {
        js_value.into_serde().unwrap()
    }
}

// Implement the `From` trait for converting a ProtocolMessage to a JsValue
impl From<ProtocolMessage> for JsValue {
    fn from(protocol_message: ProtocolMessage) -> Self {
        protocol_message.to_js_value().unwrap()
    }
}

// Implement the `From` trait for converting a JsValue to a MessageType
impl From<JsValue> for MessageType {
    fn from(js_value: JsValue) -> Self {
        let protocol_message: ProtocolMessage = js_value.into_serde().unwrap();
        protocol_message.message
    }
}

// Implement the `From` trait for converting a MessageType to a JsValue
impl From<MessageType> for JsValue {
    fn from(message_type: MessageType) -> Self {
        let protocol_message = ProtocolMessage::new(message_type);
        JsValue::from(protocol_message)
    }
}

impl From<RequestWithId> for JsValue {
    fn from(request: RequestWithId) -> Self {
        JsValue::from_serde(&request).unwrap()
    }
}

impl From<ResponseWithId> for JsValue {
    fn from(response: ResponseWithId) -> Self {
        JsValue::from_serde(&response).unwrap()
    }
}
