use js_sys::{Function, Reflect};
use wasm_bindgen::prelude::*;

// Rust wrappers

// Updated wrapper function that accesses `port.onDisconnect.addListener`
pub fn add_on_disconnect_listener(port: JsValue, callback: &Function) -> Result<(), JsValue> {
    // Retrieve the `onDisconnect` object from `port`
    let on_disconnect = Reflect::get(&port, &JsValue::from_str("onDisconnect"))?;

    // Retrieve the `addListener` function from `onDisconnect`
    let add_listener_func =
        Reflect::get(&on_disconnect, &JsValue::from_str("addListener"))?.dyn_into::<Function>()?;

    // Call `addListener` on `onDisconnect` with `callback` as the argument
    add_listener_func.call1(&on_disconnect, callback)?;

    Ok(())
}

pub fn add_on_message_listener(port: JsValue, callback: &Function) -> Result<(), JsValue> {
    // Retrieve the `onMessage` object from `port`
    let on_message = Reflect::get(&port, &JsValue::from_str("onMessage"))?;

    // Retrieve the `addListener` function from `onMessage`
    let add_listener_func =
        Reflect::get(&on_message, &JsValue::from_str("addListener"))?.dyn_into::<Function>()?;

    // Call `addListener` on `onMessage` with `callback` as the argument
    add_listener_func.call1(&on_message, callback)?;

    Ok(())
}

// Wrapper function to access `port.onDisconnect.removeListener`
pub fn remove_on_disconnect_listener(port: JsValue, callback: &Function) -> Result<(), JsValue> {
    // Retrieve the `onDisconnect` object from `port`
    let on_disconnect = Reflect::get(&port, &JsValue::from_str("onDisconnect"))?;

    // Retrieve the `removeListener` function from `onDisconnect`
    let remove_listener_func = Reflect::get(&on_disconnect, &JsValue::from_str("removeListener"))?
        .dyn_into::<Function>()?;

    // Call `removeListener` on `onDisconnect` with `callback` as the argument
    remove_listener_func.call1(&on_disconnect, callback)?;

    Ok(())
}

// TODO: Documentation on why this uses reflect versus the others that don't
pub fn post_message(port: &JsValue, message: JsValue) -> Result<(), JsValue> {
    // Retrieve `postMessage` as a Function from the port object
    let post_message_func =
        Reflect::get(port, &JsValue::from_str("postMessage"))?.dyn_into::<Function>()?;

    // Call `postMessage` on `port` with `message` as the argument
    post_message_func.call1(port, &message)?;

    Ok(())
}
