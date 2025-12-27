use crate::components::cluster::{Cluster, Row, Value};
use crate::components::general::{self, CannotConnectSub, UnsupportedOrigin};
use leptos::prelude::*;
use nexum_chrome_sys::tabs::TabData;

#[component]
pub fn UnsupportedTab(tab: ReadSignal<Option<TabData>>) -> impl IntoView {
    // Format the URL (if it exists) for display as the unsupported origin
    let url = move || {
        tab.with(|tab| {
            tab.as_ref()
                .and_then(|tab| tab.url.clone())
                .unwrap_or_else(|| "Unknown URL".to_string())
        })
    };

    view! {
        <Cluster>
            <Row>
                <Value>
                    <div style="padding-bottom: 32px;">
                        <general::UnsupportedTab>"Unsupported tab"</general::UnsupportedTab>
                        <CannotConnectSub>
                            <div>"Frame does not have access to"</div>
                            <UnsupportedOrigin>{url()}</UnsupportedOrigin>
                            <div>"tabs in this browser"</div>
                        </CannotConnectSub>
                    </div>
                </Value>
            </Row>
        </Cluster>
    }
}
