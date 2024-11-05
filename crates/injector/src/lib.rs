use nexum_primitives::ProtocolMessage;
use tracing::{debug, error, instrument, trace};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Event, HtmlScriptElement};

// Bind to `chrome` API for `runtime` and `onMessage` listener
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = chrome, js_name = runtime)]
    pub type ChromeRuntime;

    #[wasm_bindgen(js_namespace = ["chrome", "runtime"], js_name = getURL)]
    fn getURL(path: &str) -> String;

    #[wasm_bindgen(js_namespace = ["chrome", "runtime"], js_name = sendMessage)]
    fn sendMessage(message: &JsValue);

    #[wasm_bindgen(js_namespace = ["chrome", "runtime", "onMessage"], js_name = addListener)]
    pub static addListener: js_sys::Function;
}

// Function to handle onMessage from the Chrome extension runtime
#[instrument(level = "trace", skip_all, fields(payload = ?payload))]
fn handle_runtime_message(payload: JsValue) {
    let window = match web_sys::window() {
        Some(win) => win,
        None => {
            error!("No window object available.");
            return;
        }
    };

    if ProtocolMessage::is_valid(&payload) {
        if let Err(e) = window.post_message(&payload, &window.location().origin().unwrap()) {
            error!("Failed to post message to window: {:?}", e);
        }
    } else {
        debug!("Payload is not a ProtocolMessage.");
    }
}

// Listen for messages from the page and forward to background script
#[instrument(level = "debug")]
fn setup_page_message_listener() {
    let closure = Closure::wrap(Box::new(move |event: Event| {
        let span = tracing::span!(tracing::Level::TRACE, "Message Event", ?event);
        let _enter = span.enter();

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
            sendMessage(&data);
        } else {
            trace!("Message event data is not a ProtocolMessage.");
        }
    }) as Box<dyn FnMut(_)>);

    web_sys::window()
        .unwrap()
        .add_event_listener_with_callback("message", closure.as_ref().unchecked_ref())
        .expect("Failed to add event listener for 'message' event.");

    closure.forget();
}

// Inject the frame script into the page
#[instrument(level = "debug")]
fn inject_frame_script() {
    let document = match web_sys::window().unwrap().document() {
        Some(doc) => doc,
        None => {
            error!("No document object available.");
            return;
        }
    };

    let script = match document.create_element("script") {
        Ok(element) => element.dyn_into::<HtmlScriptElement>().unwrap(),
        Err(e) => {
            error!("Failed to create script element: {:?}", e);
            return;
        }
    };

    script.set_type("module");
    script.set_src(&getURL("injected.js"));

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
        .unwrap();
    if let Err(e) = top_level.append_child(&script) {
        error!("Failed to append script to DOM: {:?}", e);
    }
}

// Initialize everything
#[wasm_bindgen(start)]
// #[instrument(level = "debug")]
pub fn run() -> Result<(), JsValue> {
    // print pretty errors in wasm https://github.com/rustwasm/console_error_panic_hook
    // This is not needed for tracing_wasm to work, but it is a common tool for getting proper error line numbers for panics.
    console_error_panic_hook::set_once();

    // Add this line:
    wasm_tracing::set_as_global_default();

    let closure = Closure::wrap(Box::new(|payload: JsValue| {
        let _span = tracing::span!(
            tracing::Level::TRACE,
            "Chrome runtime message listener",
            ?payload
        )
        .entered();
        handle_runtime_message(payload);
    }) as Box<dyn FnMut(_)>);

    if let Err(e) = addListener.call1(&JsValue::NULL, closure.as_ref().unchecked_ref()) {
        error!("Failed to add runtime message listener: {:?}", e);
    }

    closure.forget();

    setup_page_message_listener();
    inject_frame_script();

    Ok(())
}
