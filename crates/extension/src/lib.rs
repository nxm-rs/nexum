use js_sys::{Function, Promise};
use jsonrpsee::{core::client::ClientT, wasm_client::WasmClientBuilder};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_wasm_bindgen::{from_value, to_value};
use std::collections::HashMap;
use tracing::{error, info, trace, warn};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;

mod inject;

// Define a request structure
#[derive(Serialize, Deserialize)]
struct EthRequest {
    pub method: String,
    pub params: Option<Vec<serde_json::Value>>,
}

// Define an event listener storage
#[wasm_bindgen]
pub struct Eip1193Provider {
    listeners: HashMap<String, Vec<Function>>,
}

#[wasm_bindgen]
impl Eip1193Provider {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Eip1193Provider {
        // Initialise tracing for logging to the console
        tracing_wasm::set_as_global_default_with_config(
            tracing_wasm::WASMLayerConfigBuilder::new()
                .set_max_level(tracing::Level::TRACE)
                .build(),
        );

        info!("EIP-1193 provider initialised");

        Eip1193Provider {
            listeners: HashMap::new(),
        }
    }

    // Implement the `request` method to return a Promise
    #[wasm_bindgen]
    pub fn request(&self, request: JsValue) -> Promise {
        let request_future = async move {
            let req: EthRequest = from_value(request.clone()).map_err(|_| {
                error!("Invalid request format");
                JsValue::from_str(format!("Invalid request format: {:?}", request).as_str())
            })?;

            // Log the method being requested
            trace!("Raw request: {:?}", request);
            trace!("Requested method: {}", req.method);
            trace!("Requested params: {:?}", req.params);

            // Initialize the JSON-RPC client for Ethereum
            let client = WasmClientBuilder::default()
                .build("ws://172.20.0.5:8545")
                .await
                .map_err(|e| {
                    JsValue::from_str(&format!("Failed to create JSON-RPC client: {}", e))
                })?;

            // Simulate responses based on the method
            let response = match req.method.as_ref() {
                "eth_chainId" => Ok(JsValue::from_str("0x1")),
                "eth_accounts" => {
                    let accounts = vec!["0x1234567890abcdef1234567890abcdef12345678"];
                    to_value(&accounts)
                        .map_err(|_| JsValue::from_str("Failed to serialize accounts"))
                }
                "eth_blockNumber" => {
                    let res: Value = client
                        .request(&req.method, req.params.unwrap())
                        .await
                        .map_err(|e| {
                            JsValue::from_str(&format!("Failed to make request: {}", e))
                        })?;

                    // Convert the response to a string, then to `JsValue`
                    let res_str = res.to_string();
                    tracing::info!("Response: {:?}", res_str);
                    Ok(JsValue::from_str(&res_str))
                }
                _ => {
                    warn!("Method not supported: {}", req.method);
                    Err(JsValue::from_str("Method not supported"))
                }
            };

            response
        };

        // Convert the future into a Promise
        future_to_promise(request_future)
    }

    // Event listener methods

    // Adds a listener for a given event type
    #[wasm_bindgen]
    pub fn on(&mut self, event_type: &str, callback: Function) {
        info!("Adding listener for event: {}", event_type);
        let listeners = self
            .listeners
            .entry(event_type.to_string())
            .or_insert(vec![]);
        listeners.push(callback);
    }

    // Removes a listener for a given event type
    #[wasm_bindgen]
    pub fn remove_listener(&mut self, event_type: &str, callback: Function) {
        info!("Removing listener for event: {}", event_type);
        if let Some(listeners) = self.listeners.get_mut(event_type) {
            listeners.retain(|cb| cb != &callback);
        }
    }

    // Trigger an event (for internal use or testing)
    #[wasm_bindgen]
    pub fn trigger_event(&self, event_type: &str) {
        info!("Triggering event: {}", event_type);
        if let Some(listeners) = self.listeners.get(event_type) {
            for listener in listeners {
                listener.call0(&JsValue::NULL).unwrap();
            }
        }
    }
}
