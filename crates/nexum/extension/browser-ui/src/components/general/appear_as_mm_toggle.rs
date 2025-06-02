use crate::components::cluster::*;
use crate::components::general::*;
// use crate::helper::toggle_local_setting;
use leptos::prelude::*;

#[component]
pub fn AppearAsMMToggle(mm_appear: ReadSignal<bool>) -> impl IntoView {
    // Function to toggle appearance mode
    let handle_toggle = move || {
        // toggle_local_setting(APPEAR_AS_MM);
    };

    view! {
        <Row>
            <Value>
                <AppearDescription mm_appear=mm_appear>
                    <span>
                        "Injecting as "
                        <span class=if mm_appear.get() {
                            "mm"
                        } else {
                            "frame"
                        }>{if mm_appear.get() { "Metamask" } else { "Frame" }}</span>
                    </span>
                </AppearDescription>
            </Value>
        </Row>
        <Row>
            <Value>
                <AppearToggle on_click=Box::new(handle_toggle)>
                    <span>
                        "Appear As "
                        <span class=if mm_appear.get() {
                            "frame"
                        } else {
                            "mm"
                        }>{if mm_appear.get() { "Frame" } else { "Metamask" }}</span> " Instead"
                    </span>
                </AppearToggle>
            </Value>
        </Row>
    }
}
