use leptos::prelude::*;
use nexum_chrome_sys::tabs::TabData;
use nexum_primitives::FrameState;
use web_sys::window;

// Import the components required for rendering the page
use crate::components::cluster::BoxMain;
use crate::components::general::{Overlay, SettingsScroll};
use crate::panels::{FrameConnected, Main};

#[component]
pub fn SettingsPage(
    tab: ReadSignal<Option<TabData>>,
    is_supported_tab: ReadSignal<bool>,
    mm_appear: ReadSignal<bool>,
    frame_state: ReadSignal<FrameState>,
) -> impl IntoView {
    // Calculate the scrollbar width using our implemented function
    let scroll_bar_width = get_scroll_bar_width();

    view! {
        // Overlay component
        <Overlay />

        // Settings scrollable area
        <SettingsScroll scroll_bar=scroll_bar_width>
            <BoxMain>
                <FrameConnected frame_state=frame_state />
            </BoxMain>
            <Main
                tab=tab
                is_supported_tab=is_supported_tab
                mm_appear=mm_appear
                frame_state=frame_state
            />
        </SettingsScroll>
    }
}

// Function to calculate the scrollbar width in pixels
fn get_scroll_bar_width() -> i32 {
    let document = window()
        .expect("no global `window` exists")
        .document()
        .expect("should have a document on window");

    // Create an inner element
    let inner = document
        .create_element("p")
        .expect("failed to create inner element");
    inner
        .set_attribute("style", "width: 100%; height: 200px;")
        .expect("failed to set inner element style");

    // Create an outer element with hidden overflow
    let outer = document
        .create_element("div")
        .expect("failed to create outer element");
    outer.set_attribute("style", "position: absolute; top: 0px; left: 0px; visibility: hidden; width: 200px; height: 150px; overflow: hidden;").expect("failed to set outer element style");

    outer
        .append_child(&inner)
        .expect("failed to append inner to outer");
    document
        .body()
        .expect("document should have a body")
        .append_child(&outer)
        .expect("failed to append outer to body");

    // Measure the width with hidden overflow
    let width_with_no_scroll = inner.client_width();
    // Set overflow to scroll and measure again
    outer
        .set_attribute("style", "overflow: scroll;")
        .expect("failed to set overflow scroll");
    let width_with_scroll = inner.client_width();

    // Clean up by removing the created elements from the DOM
    document
        .body()
        .expect("document should have a body")
        .remove_child(&outer)
        .expect("failed to remove outer from body");

    width_with_no_scroll - width_with_scroll
}
