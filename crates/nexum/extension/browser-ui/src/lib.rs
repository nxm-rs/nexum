use std::time::Duration;

use chrome_sys::{port, tabs};
use constants::EXTENSION_PORT_NAME;
use gloo_utils::format::JsValueSerdeExt;
use leptos::{prelude::*, task::spawn_local};
use leptos_meta::*;
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
    let port = chrome_sys::runtime::connect(&JsValue::from(EXTENSION_PORT_NAME)).unwrap();
    let closure = Closure::wrap(Box::new(move |state: JsValue| {
        let state: FrameState = state.into_serde().unwrap();
        debug!("Frame state: {:?}", &state);
        set_frame_state.set(state);
    }) as Box<dyn FnMut(JsValue)>);

    port::add_on_message_listener(port, closure.as_ref().unchecked_ref()).unwrap();
    closure.forget(); // Closure is retained in memory
}

fn update_current_chain_callback(active_tab: ReadSignal<Option<tabs::Info>>) -> impl Fn() {
    move || {
        let tab_clone = active_tab.get_untracked().clone();
        spawn_local(async move {
            if let Some(tab) = tab_clone
                && let Ok(message) = JsValue::from_serde(&json!({
                    "type": "embedded:action",
                    "action": { "type": "getChainId" }
                })) {
                    chrome_sys::tabs::send_message_to_tab(&tab, message)
                        .await
                        .inspect_err(|e| tracing::error!(?e, "failed to send message to tab"))
                        .ok();
                }
        });
    }
}

async fn init(
    set_active_tab: WriteSignal<Option<tabs::Info>>,
    set_mm_appear: WriteSignal<bool>,
    set_is_injected_tab: WriteSignal<bool>,
) {
    // Get and set the active tab
    let active_tab = tabs::get_active_tab().await;
    if let Some(tab) = &active_tab {
        set_active_tab(Some(tab.clone()));
        if let Some(url) = &tab.url {
            set_is_injected_tab(
                url.starts_with("https://")
                    || url.starts_with("http://")
                    || url.starts_with("file://"),
            );
        }
    }

    // Get the initial settings from the page - hard set to not appear as MM for the moment
    // let settings = helper::get_initial_settings(&active_tab).await;
    // set_mm_appear.set(settings[0]);
    set_mm_appear(false);
}

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    // Define reactive signals
    let (active_tab, set_active_tab) = signal(None::<tabs::Info>);
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
