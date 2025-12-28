use crate::components::cluster::{BoxMain, Cluster};
use crate::components::general::{AppearAsMMToggle, ChainSelect, CurrentOriginTitle};
use crate::panels::{NotConnected, UnsupportedTab};
use leptos::prelude::*;
use nexum_chrome_sys::tabs::TabData;
use nexum_primitives::FrameState;

// Define the component props
#[component]
pub fn Main(
    tab: ReadSignal<Option<TabData>>,
    is_supported_tab: ReadSignal<bool>,
    mm_appear: ReadSignal<bool>,
    frame_state: ReadSignal<FrameState>,
) -> impl IntoView {
    let is_connected = move || frame_state.with(|state| state.frame_connected.is_connected());
    let tab_fn = move || tab.get();
    let is_supported_tab = move || is_supported_tab.get();

    // View conditional rendering based on tab support, connection, and tab presence
    view! {
        {match (tab_fn(), is_supported_tab(), is_connected()) {
            (None, _, _) | (Some(_), false, _) => {
                // Case 1: Tab is unsupported or tab is `None`
                view! {
                    <BoxMain style="margin-top: 12px;".into()>
                        <UnsupportedTab tab=tab />
                        {if !is_connected() {
                            Some(view! { <NotConnected /> })
                        } else {
                            None
                        }}
                    </BoxMain>
                }
            }
            (Some(_), true, false) => {

                // Case 2: Tab is supported but not connected
                view! {
                    <BoxMain style="margin-top: 12px;".into()>
                        <NotConnected />
                    </BoxMain>
                }
            }
            (Some(_), true, true) => {

                // Case 3: Tab is supported and connected
                view! {
                    <BoxMain style="margin-top: 12px;".into()>
                        <CurrentOriginTitle tab=tab />

                        <Cluster>
                            // Show ChainSelect if there are available chains
                            {move || {
                                if frame_state.get().available_chains.is_empty() {
                                    None
                                } else {
                                    Some(
                                        view! {
                                            <>
                                                <ChainSelect tab=tab frame_state=frame_state />
                                                <div style="height: 9px;"></div>
                                            </>
                                        },
                                    )
                                }
                            }} <AppearAsMMToggle mm_appear=mm_appear />
                        </Cluster>
                    </BoxMain>
                }
            }
        }}
    }
}
