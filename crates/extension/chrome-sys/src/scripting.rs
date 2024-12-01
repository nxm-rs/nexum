use gloo_utils::format::JsValueSerdeExt;
use js_sys::{Array, Object};
use serde::Deserialize;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

use crate::tabs;

#[wasm_bindgen(module = "/src/inject.js")]
extern "C" {
    fn createFunction(funcSource: &str) -> JsValue;
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["chrome", "scripting"], js_name = executeScript)]
    pub fn execute_script_js(script: &JsValue) -> js_sys::Promise;
}

#[derive(Deserialize)]
pub struct InjectionResult {
    pub frame_id: u32,
    pub result: Option<String>,
    // Include `error` field handling once Chrome provides support
}


pub async fn execute_script(
    tab: &tabs::Info,
    func: &str,
    args: Vec<JsValue>,
) -> Result<Vec<InjectionResult>, JsValue> {
    // Convert `args` into a `js_sys::Array`
    let js_args = args.into_iter().collect::<Array>();

    // Create the function object from `func`
    let func_object = createFunction(func);

    // Create the `details` object manually
    let details = Object::new();
    js_sys::Reflect::set(
        &details,
        &JsValue::from_str("target"),
        &JsValue::from_serde(&serde_json::json!({
            "tabId": tab.id,
        }))
        .map_err(|e| JsValue::from_str(&e.to_string()))?,
    )?;
    js_sys::Reflect::set(
        &details,
        &JsValue::from_str("func"),
        &func_object,
    )?;
    js_sys::Reflect::set(&details, &JsValue::from_str("args"), &js_args)?;

    // Await the promise from `execute_script_js` and handle errors
    match JsFuture::from(execute_script_js(&details)).await {
        Ok(result) => {
            // Deserialize the result into `Vec<InjectionResult>`
            result
                .into_serde()
                .map_err(|e| JsValue::from_str(&e.to_string()))
        }
        Err(e) => Err(e),
    }
}
