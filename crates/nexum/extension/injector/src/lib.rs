#![cfg(target_arch = "wasm32")]

use nexum_primitives::ProtocolMessage;
use tracing::{debug, error, trace};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{Event, HtmlScriptElement};

// Function to handle onMessage from the Chrome extension runtime
fn handle_runtime_message(payload: JsValue) {
    let window = match web_sys::window() {
        Some(win) => win,
        None => {
            error!("No window object available.");
            return;
        }
    };

    if ProtocolMessage::is_valid(&payload) {
        if let Err(e) = window.post_message(
            &payload,
            &window
                .location()
                .origin()
                .expect("Failed to get window origin."),
        ) {
            error!("Failed to post message to window: {:?}", e);
        }
    } else {
        debug!("Payload is not a ProtocolMessage.");
    }
}

// Listen for messages from the page and forward to background script
fn setup_page_message_listener() {
    let closure = Closure::wrap(Box::new(move |event: Event| {
        let event = match event.dyn_ref::<web_sys::MessageEvent>() {
            Some(ev) => ev,
            None => {
                error!("Failed to cast event to MessageEvent.");
                return;
            }
        };

        if event.source().is_none() {
            trace!("Message event source is None.");
            return;
        }

        let data = event.data();
        if ProtocolMessage::is_valid(&data) {
            chrome_sys::runtime::send_message(&data)
                .inspect_err(|e| tracing::error!(?e, "failed to send message to background script"))
                .ok();
        } else {
            trace!("Message event data is not a ProtocolMessage.");
        }
    }) as Box<dyn FnMut(_)>);

    web_sys::window()
        .expect("Failed to get window object.")
        .add_event_listener_with_callback("message", closure.as_ref().unchecked_ref())
        .expect("Failed to add event listener for 'message' event.");

    closure.forget();
}

// Inject the frame script into the page
fn inject_frame_script() {
    let document = match web_sys::window()
        .expect("Failed to get window in inject_frame_script")
        .document()
    {
        Some(doc) => doc,
        None => {
            error!("No document object available.");
            return;
        }
    };

    let script = match document.create_element("script") {
        Ok(element) => element
            .dyn_into::<HtmlScriptElement>()
            .expect("Failed to cast element to script element."),
        Err(e) => {
            error!("Failed to create script element: {:?}", e);
            return;
        }
    };

    script.set_type("module");
    script.set_src(&chrome_sys::runtime::getURL("injected.js"));

    let script_clone = script.clone();
    let onload = Closure::wrap(Box::new(move || {
        let _span = tracing::span!(tracing::Level::DEBUG, "Frame script onload").entered();

        if let Some(parent) = script_clone.parent_node() {
            if let Err(e) = parent.remove_child(&script_clone) {
                error!("Failed to remove script from DOM: {:?}", e);
            }
        }
    }) as Box<dyn FnMut()>);

    script.set_onload(Some(onload.as_ref().unchecked_ref()));
    onload.forget();

    let top_level = document
        .head()
        .map(|e| e.into())
        .or_else(|| document.document_element())
        .expect("Failed to get top level element.");
    if let Err(e) = top_level.append_child(&script) {
        error!("Failed to append script to DOM: {:?}", e);
    }
}

// Initialize everything
#[wasm_bindgen(start)]
pub fn run() -> Result<(), JsValue> {
    // print pretty errors in wasm https://github.com/rustwasm/console_error_panic_hook
    // This is not needed for tracing_wasm to work, but it is a common tool for getting proper error line numbers for panics.
    console_error_panic_hook::set_once();

    // Add this line:
    wasm_tracing::set_as_global_default();

    let closure = Closure::wrap(Box::new(|payload: JsValue| {
        handle_runtime_message(payload);
    }) as Box<dyn FnMut(_)>);

    chrome_sys::runtime::add_on_message_listener(closure.as_ref().unchecked_ref());

    closure.forget();

    setup_page_message_listener();
    inject_frame_script();

    Ok(())
}
