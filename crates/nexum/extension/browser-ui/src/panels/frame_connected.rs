use crate::components::cluster::{Cluster, Row, Value};
use crate::components::general::{self, LogoWrap, SummonFrameButton};
use crate::constants::FRAME_SUMMON;
use gloo_utils::format::JsValueSerdeExt;
use leptos::*;
use nexum_primitives::{ConnectionState, FrameState};
use serde_json::json;
use wasm_bindgen::JsValue;

#[component]
pub fn FrameConnected(frame_state: ReadSignal<FrameState>) -> impl IntoView {
    let is_connected = move || frame_state.with(|state| state.frame_connected.is_connected());
    // Define the click handler for the summon frame action
    let handle_summon_frame = Box::new(move || {
        if is_connected() {
            // Send a message to summon the frame if connected
            if let Ok(message) = JsValue::from_serde(&json!({
                "method": FRAME_SUMMON,
                "params": [],
            })) {
                chrome_sys::runtime::send_message(&message);
            }
        }
    });

    let (style, logo_src, connection_color) = match is_connected() {
        true => (
            "flex-grow: 1; color: var(--good); justify-content: space-between; height: 64px;",
            "../icons/icon96good.png",
            "color: var(--good);",
        ),
        false => (
            "flex-grow: 1; color: var(--moon); justify-content: space-between; height: 64px;",
            "../icons/icon96moon.png",
            "color: var(--moon);",
        ),
    };

    view! {
        <Cluster>
            <Row>
                <Value style=style.to_string() on_interact=handle_summon_frame>
                    <LogoWrap src=logo_src.to_string() alt="Connection status".to_string() />
                    <general::FrameConnected style=connection_color
                        .to_string()
                        .into()>
                        {move || {
                            if frame_state.get().frame_connected == ConnectionState::Connected {
                                "Frame Connected"
                            } else {
                                "Frame Disconnected"
                            }
                        }}
                    </general::FrameConnected>
                    <SummonFrameButton />
                </Value>
            </Row>
        </Cluster>
    }
}
