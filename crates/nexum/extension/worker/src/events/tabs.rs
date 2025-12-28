use std::sync::Arc;

use nexum_chrome_gloo::tabs::{self, OnActivatedActiveInfo, OnUpdatedChangeInfo};
use tracing::{debug, trace, warn};
use wasm_bindgen::prelude::*;

use crate::{Extension, origin_from_url, state::tab_unsubscribe};

// To be used with the `chrome.tabs.onRemoved` event
pub async fn tabs_on_removed(extension: Arc<Extension>, tab_id: JsValue) {
    let tab_id: u32 = tab_id.as_f64().unwrap() as u32;
    trace!(tab_id, "Tab removed");

    let mut state = extension.state.lock().await;
    state.tab_origins.remove(&tab_id);

    // Attempt to unsubscribe the tab and log if it fails
    if let Err(e) = tab_unsubscribe(extension.clone(), tab_id).await {
        warn!(tab_id, error = ?e, "Failed to unsubscribe tab");
    }
}

// Handler for `chrome.tabs.onUpdated` event
pub async fn tabs_on_updated(extension: Arc<Extension>, tab_id: JsValue, change_info: JsValue) {
    trace!("Received tab update event: {:?}", change_info);
    let tab_id: u32 = tab_id.as_f64().unwrap() as u32;
    let change_info: OnUpdatedChangeInfo = change_info.unchecked_into();

    // Trace tab update and check for URL changes
    let url = change_info.get_url();
    trace!(tab_id, ?url, "Tab updated");

    if let Some(url) = url {
        let origin = origin_from_url(Some(url));
        debug!(tab_id, ?origin, "Updated tab origin");

        let mut state = extension.state.lock().await;
        if let Some(existing_origin) = state.tab_origins.get(&tab_id) {
            if *existing_origin != origin {
                state.tab_origins.insert(tab_id, origin);

                // Attempt to unsubscribe the tab and log if it fails
                if let Err(e) = tab_unsubscribe(extension.clone(), tab_id).await {
                    warn!(tab_id, error = ?e, "Failed to unsubscribe tab");
                }
            }
        } else {
            state.tab_origins.insert(tab_id, origin);
        }
    } else {
        trace!(tab_id, "No URL change detected for tab");
    }
}

// Handler for `chrome.tabs.onActivated` event
pub async fn tabs_on_activated(extension: Arc<Extension>, active_info: JsValue) {
    let active_info: OnActivatedActiveInfo = active_info.unchecked_into();
    let tab_id = active_info.get_tab_id();

    let _tab = match tabs::get(tab_id).await {
        Ok(tab) => tab,
        Err(e) => {
            warn!(tab_id, error = ?e, "Failed to get tab");
            return;
        }
    };

    // Update the active tab ID
    let mut state = extension.state.lock().await;
    state.active_tab_id = Some(tab_id as u32);
    debug!(active_tab_id = ?state.active_tab_id, "Updated active tab ID");
}
