#![cfg(target_arch = "wasm32")]

use js_sys::{Object, Reflect};
use provider::EthereumProvider;
use wasm_bindgen::{JsValue, prelude::wasm_bindgen};
use web_sys::window;

mod eip6963;
mod provider;

macro_rules! set_properties {
    ($object:expr, { $($key:literal : $value:expr),* $(,)? }) => {
        $(
            Reflect::set(&$object, &JsValue::from_str($key), &$value)?;
        )*
    };
}

#[wasm_bindgen]
pub fn initialize_provider() -> Result<(), JsValue> {
    // print pretty errors in wasm https://github.com/rustwasm/console_error_panic_hook
    // This is not needed for tracing_wasm to work, but it is a common tool for getting proper error line numbers for panics.
    console_error_panic_hook::set_once();

    // Add this line:
    wasm_tracing::set_as_global_default();

    // Create a new instance of the EthereumProvider
    let provider = EthereumProvider::new();

    // Access the global `window` object
    let window = window().ok_or_else(|| JsValue::from_str("No global window object"))?;

    // Convert the provider to a JsValue to pass it to JavaScript
    let provider_js = JsValue::from(provider);

    // Check if `window.ethereum` already exists
    let ethereum_descriptor = Reflect::get_own_property_descriptor(&window, &"ethereum".into())?;

    if !ethereum_descriptor.is_undefined() {
        // If `window.ethereum` exists, check if it is configurable
        if Reflect::get(&ethereum_descriptor, &"configurable".into())?
            .as_bool()
            .unwrap_or(false)
        {
            // Create a property descriptor and set the necessary properties using the macro
            let property_descriptor = Object::new();
            set_properties!(property_descriptor, {
                "value": provider_js.clone(),
                "writable": JsValue::TRUE,
                "configurable": JsValue::TRUE,
                "enumerable": JsValue::TRUE,
            });

            // Define `window.ethereum` with the descriptor settings
            Reflect::define_property(&window, &"ethereum".into(), &property_descriptor)?;
        } else {
            // If not configurable, set `window.ethereum` directly
            Reflect::set(&window, &"ethereum".into(), &provider_js)?;
        }
    } else {
        // If `window.ethereum` does not exist, set it directly
        Reflect::set(&window, &"ethereum".into(), &provider_js)?;
    }

    Ok(())
}
