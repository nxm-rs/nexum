use js_sys::{Function, Reflect};
use wasm_bindgen::prelude::*;

// Rust wrappers

// Updated wrapper function that accepts a pre-made `&Function` reference
pub fn port_add_on_disconnect_listener(port: JsValue, callback: &Function) -> Result<(), JsValue> {
    // Retrieve the `addListener` function from the `port` object
    let add_listener_func = Reflect::get(&port, &JsValue::from_str("addListener"))?
        .dyn_into::<Function>()?;

    // Call `addListener` on `port` with `callback` as the argument
    add_listener_func.call1(&port, callback)?;

    Ok(())
}

pub fn remove_on_disconnect_listener(port: JsValue, callback: &Function) -> Result<(), JsValue> {
    // Retrieve the `removeListener` function from the `port` object
    let remove_listener_func = Reflect::get(&port, &JsValue::from_str("removeListener"))?
        .dyn_into::<Function>()?;

    // Call `removeListener` on `port` with `callback` as the argument
    remove_listener_func.call1(&port, callback)?;

    Ok(())
}

// TODO: Documentation on why this uses reflect versus the others that don't
pub fn post_message(port: &JsValue, message: JsValue) -> Result<(), JsValue> {
    // Retrieve `postMessage` as a Function from the port object
    let post_message_func = Reflect::get(port, &JsValue::from_str("postMessage"))?
        .dyn_into::<Function>()?;
    
    // Call `postMessage` on `port` with `message` as the argument
    post_message_func.call1(port, &message)?;

    Ok(())
}