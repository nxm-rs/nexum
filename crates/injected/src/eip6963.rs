use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct EIP6963ProviderInfo {
    pub uuid: String,
    pub name: String,
    pub icon: String,
    pub rdns: String,
}

// Define the EIP6963Provider trait
pub(crate) trait EIP6963Provider {
    fn get_info(&self) -> EIP6963ProviderInfo;
}

// Detail structure for EIP6963, holding provider information in JsValue format for compatibility
#[wasm_bindgen]
#[derive(Debug)]
pub(crate) struct EIP6963ProviderDetail {
    info: JsValue,
    provider: JsValue,
}

#[wasm_bindgen]
impl EIP6963ProviderDetail {
    // Constructor to create EIP6963ProviderDetail with serialized info
    #[wasm_bindgen(constructor)]
    pub fn new(info: JsValue, provider: JsValue) -> Self {
        EIP6963ProviderDetail { info, provider }
    }

    // Getter for `info`
    #[wasm_bindgen(getter)]
    pub fn info(&self) -> JsValue {
        self.info.clone()
    }

    // Getter for `provider`
    #[wasm_bindgen(getter)]
    pub fn provider(&self) -> JsValue {
        self.provider.clone()
    }
}
