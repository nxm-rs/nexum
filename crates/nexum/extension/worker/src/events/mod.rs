mod alarm;
use std::sync::Arc;

use alarm::*;
mod idle;
use chrome_sys::tabs::Info;
use idle::*;
mod runtime;
use gloo_utils::format::JsValueSerdeExt;
use nexum_primitives::{EthEvent, MessageType, ProtocolMessage};
use runtime::*;
mod tabs;
use tabs::*;

use crate::Extension;
use serde_wasm_bindgen::from_value;
use tracing::{trace, warn};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

pub(crate) fn setup_listeners(extension: Arc<Extension>) -> Result<(), JsValue> {
    // Clone the Arc references once at the start
    let extension_clone = extension.clone();

    // Runtime `on_message` event
    let closure = {
        let extension = extension_clone.clone();
        Closure::wrap(Box::new(move |payload: JsValue, sender: JsValue| {
            let extension = extension.clone();
            trace!("runtime::on_message_add_listener: {:?}", payload);
            spawn_local(async move {
                runtime_on_message(extension, payload, sender).await;
            });
        }) as Box<dyn FnMut(JsValue, JsValue)>)
    };
    chrome_sys::runtime::add_on_message_listener(closure.as_ref().unchecked_ref());
    closure.forget();

    // Runtime `on_connect` event
    let closure = {
        let extension = extension_clone.clone();
        Closure::wrap(Box::new(move |port: JsValue| {
            let extension = extension.clone();
            trace!("runtime::on_connect_add_listener: {:?}", port);
            spawn_local(async move {
                runtime_on_connect(extension, port).await;
            });
        }) as Box<dyn FnMut(JsValue)>)
    };
    chrome_sys::runtime::add_on_connect_listener(closure.as_ref().unchecked_ref());
    closure.forget();

    // Idle `on_state_changed` event
    let closure = {
        let extension = extension_clone.clone();
        Closure::wrap(Box::new(move |state: JsValue| {
            let extension = extension.clone();
            trace!("idle::on_state_changed_add_listener: {:?}", state);
            spawn_local(async move {
                idle_on_state_changed(extension, state).await;
            });
        }) as Box<dyn FnMut(JsValue)>)
    };
    chrome_sys::idle::on_state_changed_add_listener(closure.as_ref().unchecked_ref());
    closure.forget();

    // Tabs `on_updated` event
    let closure = {
        let extension = extension_clone.clone();
        Closure::wrap(
            Box::new(move |tab_id: JsValue, change_info: JsValue, tab: JsValue| {
                let extension = extension.clone();
                trace!(
                    "Tab updated: tab_id={:?}, change_info={:?}, tab={:?}",
                    tab_id, change_info, tab
                );
                spawn_local(async move {
                    tabs_on_updated(extension, tab_id, change_info).await;
                });
            }) as Box<dyn FnMut(JsValue, JsValue, JsValue)>,
        )
    };
    chrome_sys::tabs::add_tab_updated_listener(closure.as_ref().unchecked_ref());
    closure.forget();

    // Tabs `on_activated` event
    let closure = {
        let extension = extension_clone.clone();
        Closure::wrap(Box::new(move |active_info: JsValue| {
            let extension = extension.clone();
            trace!("tabs::on_activated_add_listener: {:?}", active_info);
            spawn_local(async move {
                tabs_on_activated(extension, active_info).await;
            });
        }) as Box<dyn FnMut(JsValue)>)
    };
    chrome_sys::tabs::add_tab_activated_listener(closure.as_ref().unchecked_ref());
    closure.forget();

    // Tabs `on_removed` event
    let closure = {
        let extension = extension_clone.clone();
        Closure::wrap(Box::new(move |tab_id: JsValue| {
            let extension = extension.clone();
            trace!("tabs::on_removed_add_listener: {:?}", tab_id);
            spawn_local(async move {
                tabs_on_removed(extension, tab_id).await;
            });
        }) as Box<dyn FnMut(JsValue)>)
    };
    chrome_sys::tabs::add_tab_removed_listener(closure.as_ref().unchecked_ref());
    closure.forget();

    // Alarms `on_alarm` event
    let closure = {
        trace!("Setting up alarms event listener");
        let extension = extension_clone.clone();
        Closure::wrap(Box::new(move |alarm: JsValue| {
            let extension = extension.clone();
            trace!("alarms::on_alarm_add_listener: {:?}", alarm);
            spawn_local(async move {
                on_alarm(extension, alarm).await;
            });
        }) as Box<dyn FnMut(JsValue)>)
    };
    chrome_sys::alarms::add_alarm_listener(closure.as_ref().unchecked_ref());
    closure.forget();

    Ok(())
}

// Send an event to a specific tab
async fn send_event_to_tab(tab: &Info, event: String, args: JsValue) -> Result<(), JsValue> {
    // Attempt to send the message to the tab
    chrome_sys::tabs::send_message_to_tab(
        tab,
        JsValue::from(ProtocolMessage::new(MessageType::EthEvent(EthEvent {
            event: event.to_string(),
            args: from_value(args)?,
        }))),
    )
    .await
    .map(|_| ())
}

// Generalized `send_event` function to handle any array type for args
pub(crate) async fn send_event(
    event: &'static str,
    args: Option<JsValue>, // Pass JsValue directly, defaulting to empty object if None
    selector: chrome_sys::tabs::Query,
) {
    // Query tabs based on the provided selector
    let tabs_js = match chrome_sys::tabs::query(selector).await {
        Ok(tabs) => tabs,
        Err(e) => {
            warn!("Failed to query tabs: {:?}", e);
            return;
        }
    };

    // Convert to Vec<tabs::Info> and default to an empty array on error
    let tabs: Vec<chrome_sys::tabs::Info> = from_value(tabs_js).unwrap_or_default();
    // Define args_js as the provided JsValue or an empty array if None, and make sure to include type annotations
    let args_js =
        args.unwrap_or_else(|| JsValue::from_serde::<&[serde_json::Value; 0]>(&&[]).unwrap());

    trace!(event, tab_count = tabs.len(), "Sending event to tabs");

    // Filter tabs with valid `id` and `url`, then send the event to each
    spawn_local(async move {
        for tab in tabs.iter().filter(|tab| tab.valid()) {
            if let Err(e) = send_event_to_tab(tab, event.to_owned(), args_js.clone()).await {
                warn!(error = ?e, "Failed to send event to tab: {:?}", tab);
            } else {
                trace!("Event sent successfully to tab: {:?}", tab);
            }
        }
    });
}
