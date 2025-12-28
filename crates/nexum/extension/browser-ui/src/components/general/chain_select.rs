use crate::components::cluster::Row;
use crate::components::general::ChainButton;
use leptos::prelude::*;
use nexum_chrome_sys::tabs::TabData;
use nexum_primitives::FrameState;

// Define props for the component
#[component]
pub fn ChainSelect(
    tab: ReadSignal<Option<TabData>>,
    frame_state: ReadSignal<FrameState>,
) -> impl IntoView {
    // Render the component
    view! {
        {
            let rows = frame_state
                .with(|state| {
                    let mut rows = Vec::new();
                    for pair in state.available_chains.keys().collect::<Vec<_>>().chunks(2) {
                        let chain_buttons = pair
                            .iter()
                            .enumerate()
                            .map(|(i, chain)| {

                                view! {
                                    <ChainButton
                                        chain=*(*chain)
                                        frame_state=frame_state
                                        index=i
                                        tab=tab
                                    />
                                }
                            })
                            .collect::<Vec<_>>();
                        rows.push(

                            view! {
                                <Row style="justify-content: flex-start;"
                                    .into()>{chain_buttons}</Row>
                            },
                        );
                    }
                    rows
                });
            rows.into_view()
        }
    }
}
