use crate::components::cluster::{Cluster, Row, Value};
use crate::components::general::{self, CannotConnectSub, Download};
use leptos::*;

#[component]
pub fn NotConnected() -> impl IntoView {
    view! {
        <Cluster>
            <Row>
                <Value>
                    <div style="padding-bottom: 32px;">
                        <general::NotConnected>"Unable to connect to Frame"</general::NotConnected>
                        <CannotConnectSub>
                            "Make sure the Frame desktop app is running"
                        </CannotConnectSub>
                        <CannotConnectSub>"on your machine or download it below"</CannotConnectSub>
                    </div>
                </Value>
            </Row>
            <Row>
                <Value pointer_events=true>
                    <Download href="https://frame.sh".to_string() target="_newtab".to_string()>
                        "Download Frame"
                    </Download>
                </Value>
            </Row>
        </Cluster>
    }
}
