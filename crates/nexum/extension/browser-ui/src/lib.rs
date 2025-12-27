use std::time::Duration;

use constants::EXTENSION_PORT_NAME;
use gloo_utils::format::JsValueSerdeExt;
use js_sys::Object;
use leptos::{prelude::*, task::spawn_local};
use leptos_meta::*;
use nexum_chrome_gloo::events::EventListener2;
use nexum_chrome_gloo::tabs;
use nexum_chrome_sys::runtime;
use nexum_chrome_sys::tabs::TabData;
use nexum_primitives::FrameState;
use pages::settings::SettingsPage;
use send_wrapper::SendWrapper;
use serde_json::json;
use tracing::debug;
use wasm_bindgen::prelude::*;

// Modules
mod components;
mod constants;
mod helper;
mod pages;
mod panels;

// Define a function to connect to the frame
fn frame_connect(set_frame_state: WriteSignal<FrameState>) {
    // Create connect info with the port name
    let connect_info = Object::new();
    js_sys::Reflect::set(
        &connect_info,
        &JsValue::from_str("name"),
        &JsValue::from_str(EXTENSION_PORT_NAME),
    )
    .unwrap();
    let port = runtime::connect(None, Some(connect_info));

    // Set up message listener using EventListener2 (receives message and port)
    let listener =
        EventListener2::new(&port.on_message(), move |state: JsValue, _port: JsValue| {
            if let Ok(state) = state.into_serde::<FrameState>() {
                debug!("Frame state: {:?}", &state);
                set_frame_state.set(state);
            }
        });

    match listener {
        Ok(l) => l.forget(), // Keep listener alive
        Err(e) => tracing::error!(?e, "Failed to add onMessage listener"),
    }
}

fn update_current_chain_callback(active_tab: ReadSignal<Option<TabData>>) -> impl Fn() {
    move || {
        let tab_clone = active_tab.get_untracked();
        spawn_local(async move {
            if let Some(tab) = tab_clone
                && let Some(tab_id) = tab.id
                && let Ok(message) = JsValue::from_serde(&json!({
                    "type": "embedded:action",
                    "action": { "type": "getChainId" }
                }))
            {
                if let Err(e) = tabs::send_message(tab_id, message).await {
                    tracing::error!(?e, "failed to send message to tab");
                }
            }
        });
    }
}

async fn init(
    set_active_tab: WriteSignal<Option<TabData>>,
    set_mm_appear: WriteSignal<bool>,
    set_is_injected_tab: WriteSignal<bool>,
) {
    // Get and set the active tab
    if let Ok(Some(tab)) = tabs::get_active_tab().await {
        let tab_data: TabData = (&tab).into();
        if let Some(ref url) = tab_data.url {
            set_is_injected_tab(
                url.starts_with("https://")
                    || url.starts_with("http://")
                    || url.starts_with("file://"),
            );
        }
        set_active_tab(Some(tab_data));
    }

    // Get the initial settings from the page - hard set to not appear as MM for the moment
    set_mm_appear(false);
}

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    // Define reactive signals
    let (active_tab, set_active_tab) = signal(None::<TabData>);
    let (mm_appear, set_mm_appear) = signal(false);
    let (is_injected_tab, set_is_injected_tab) = signal(false);

    let (frame_state, set_frame_state) = signal(FrameState::default());

    // Set up frame connection
    frame_connect(set_frame_state);

    // Set up the 1-second interval for updating the current chain
    let interval = set_interval_with_handle(
        update_current_chain_callback(active_tab),
        Duration::from_secs(1),
    )
    .expect("failed to set interval");

    // Automatically clear the interval when the component is cle/workspaces/ferris/crates/browser-uianed up
    on_cleanup(move || {
        interval.clear();
    });

    view! {
        <Await
            // `future` provides the `Future` to be resolved
            future=SendWrapper::new(init(set_active_tab, set_mm_appear, set_is_injected_tab))
            // the data is bound to whatever variable name you provide
            let:_data
        >
            <Html attr:lang="en" attr:dir="ltr" attr:data-theme="light" />

            // sets the document title
            <Title text="Welcome to Leptos CSR" />

            // injects metadata in the <head> of the page
            <Meta charset="UTF-8" />
            <Meta name="viewport" content="width=device-width, initial-scale=1.0" />

            // if active tab is set, render the settings page, otherwise render an error message
            // do not use suspense here
            <SettingsPage
                tab=active_tab
                is_supported_tab=is_injected_tab
                mm_appear=mm_appear
                frame_state=frame_state
            />
        </Await>
    }
}
