mod alarm;
use alarm::*;
mod idle;
use idle::*;
mod runtime;
use runtime::*;
mod tabs;
use tabs::*;

use std::{cell::RefCell, rc::Rc};

use crate::Extension;
use ferris_primitives::{EthEventPayload, MessagePayload};
use jsonrpsee::wasm_client::Client;
use serde_wasm_bindgen::from_value;
use tracing::{trace, warn};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

pub(crate) fn setup_listeners(extension: Rc<RefCell<Extension>>, provider: Rc<RefCell<Client>>) {
    // Runtime `on_message` event

    let closure = Closure::wrap(Box::new(move |payload: JsValue, sender: JsValue| {
        let provider_clone = provider.clone();
        trace!("runtime::on_message_add_listener: {:?}", payload);
        spawn_local(async move {
            runtime_on_message(provider_clone, payload, sender).await;
        });
    }) as Box<dyn FnMut(JsValue, JsValue)>);
    chrome_sys::runtime::on_message_add_listener(closure.as_ref().unchecked_ref());
    closure.forget();

    // Runtime `on_connect` event
    let extension_clone = extension.clone();
    let closure = Closure::wrap(Box::new(move |port: JsValue| {
        let extension_clone = extension_clone.clone();
        trace!("runtime::on_connect_add_listener: {:?}", port);
        spawn_local(async move {
            runtime_on_connect(extension_clone, port).await;
        });
    }) as Box<dyn FnMut(JsValue)>);
    chrome_sys::runtime::on_connect_add_listener(closure.as_ref().unchecked_ref());
    closure.forget();

    // Idle `on_state_changed` event
    let extension_clone = extension.clone();
    let closure = Closure::wrap(Box::new(move |state: JsValue| {
        let extension_clone = extension_clone.clone();
        trace!("idle::on_state_changed_add_listener: {:?}", state);
        spawn_local(async move {
            idle_on_state_changed(extension_clone, state).await;
        });
    }) as Box<dyn FnMut(JsValue)>);
    chrome_sys::idle::on_state_changed_add_listener(closure.as_ref().unchecked_ref());
    closure.forget();

    // Tabs `on_updated` event
    let extension_clone = extension.clone();
    let closure = Closure::wrap(Box::new(
        move |tab_id: JsValue, change_info: JsValue, tab: JsValue| {
            let extension_clone = extension_clone.clone();
            trace!(
                "Tab updated: tab_id={:?}, change_info={:?}, tab={:?}",
                tab_id,
                change_info,
                tab
            );
            tabs_on_updated(extension_clone, tab_id, change_info);
        },
    ) as Box<dyn FnMut(JsValue, JsValue, JsValue)>);
    chrome_sys::tabs::on_updated_add_listener(closure.as_ref().unchecked_ref());
    closure.forget();

    // Tabs `on_activated` event
    let extension_clone = extension.clone();
    let closure = Closure::wrap(Box::new(move |tab_id: JsValue| {
        let extension_clone = extension_clone.clone();
        trace!("tabs::on_removed_add_listener: {:?}", tab_id);
        spawn_local(async move {
            tabs_on_removed(extension_clone, tab_id).await;
        });
    }) as Box<dyn FnMut(JsValue)>);
    chrome_sys::tabs::on_removed_add_listener(closure.as_ref().unchecked_ref());
    closure.forget();

    // Tabs `on_removed` event
    let extension_clone = extension.clone();
    let closure = Closure::wrap(Box::new(move |active_info: JsValue| {
        let extension_clone = extension_clone.clone();
        trace!("tabs::on_activated_add_listener: {:?}", active_info);
        spawn_local(async move {
            tabs_on_activated(extension_clone, active_info).await;
        });
    }) as Box<dyn FnMut(JsValue)>);
    chrome_sys::tabs::on_activated_add_listener(closure.as_ref().unchecked_ref());
    closure.forget();


    // Alarms `on_alarm` event
    let extension_clone = extension.clone();
    let closure = Closure::wrap(Box::new(move |alarm: JsValue| {
        let extension_clone = extension_clone.clone();
        trace!("alarms::on_alarm_add_listener: {:?}", alarm);
        alarms_on_alarm(extension_clone, alarm);
    }) as Box<dyn FnMut(JsValue)>);
    chrome_sys::alarms::on_alarm_add_listener(closure.as_ref().unchecked_ref());
    closure.forget();
}

// Send an event to a specific tab
async fn send_event_to_tab(tab_id: u32, event: String, args: JsValue) -> Result<(), JsValue> {
    let event = MessagePayload::EthEvent(EthEventPayload::new(event, args));
    chrome_sys::tabs::send_message_to_tab(tab_id, event.to_js_value())
        .await
        .map_err(|e| {
            warn!(
                "Error sending event \"{}\" to tab {}: {:?}",
                event, tab_id, e
            );
            JsValue::from_str("Error sending message to tab")
        })?;

    Ok(())
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
    let args_js = args.unwrap_or_else(JsValue::undefined);

    trace!(event, tab_count = tabs.len(), "Sending event to tabs");

    // Filter tabs with valid `id` and `url`, then send the event to each
    spawn_local(async move {
        for tab in tabs.iter().filter(|tab| tab.valid()) {
            let tab_id = tab.id.expect("Tab id should exist after filtering");

            if let Err(e) = send_event_to_tab(tab_id, event.to_owned(), args_js.clone()).await {
                warn!(tab_id, error = ?e, "Failed to send event to tab");
            } else {
                trace!(tab_id, "Event sent successfully to tab");
            }
        }
    });
}
