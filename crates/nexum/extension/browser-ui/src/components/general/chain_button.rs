use crate::components::cluster::Value;
use crate::components::general::{ChainButtonIcon, ChainButtonLabel};
use crate::helper::update_current_chain;
use alloy_chains::Chain;
use chrome_sys::tabs;
use gloo_utils::format::JsValueSerdeExt;
use leptos::*;
use nexum_primitives::FrameState;
use serde_json::json;
use wasm_bindgen::JsValue;

#[component]
pub fn ChainButton(
    chain: Chain,
    frame_state: ReadSignal<FrameState>,
    index: usize,
    tab: ReadSignal<Option<tabs::Info>>,
) -> impl IntoView {
    // A chain can be selected if it is connected and a tab is available
    let is_selectable = frame_state.with(|state| {
        state
            .available_chains
            .get(&chain)
            .map(|connection_state| connection_state.is_disconnected())
            .unwrap_or(false)
            && state
                .current_chain_in_tab
                .map(|current_chain| current_chain != chain)
                .unwrap_or(true)
    }) && tab.get().is_some();

    let selected = frame_state.with(|state| {
        state
            .current_chain_in_tab
            .map(|current_chain| current_chain == chain)
            .unwrap_or(false)
    });

    // Define the handle_click function
    let handle_click = Box::new(move || {
        if is_selectable {
            // Send message to switch Ethereum chain
            if let Ok(message) = JsValue::from_serde(&json!({
                "tab": tab.get().as_ref().unwrap().id.unwrap(),
                "method": "wallet_switchEthereumChain",
                "params": [{"chainId": chain.id()}],
            })) {
                chrome_sys::runtime::send_message(&message);
                let tab_clone = tab.clone();
                spawn_local(async move {
                    update_current_chain(&tab.get()).await;
                });
            }
        }
    });

    // Dynamic styling for the component
    let style = format!(
        "flex-grow: 0; width: calc(50% - 3px); border-bottom-right-radius: {}; opacity: {}; cursor: {}",
        if index == 0 { "8px" } else { "auto" },
        if is_selectable { "1" } else { "0.4" },
        if is_selectable { "pointer" } else { "default" },
    );

    view! {
        <Value style=style on_interact=handle_click>
            <ChainButtonIcon selected=selected />
            <ChainButtonLabel>{chain.to_string()}</ChainButtonLabel>
        </Value>
    }
}
