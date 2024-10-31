use crate::get_extension;
use chrome_sys::{alarms, idle, runtime, tabs};
use ferris_primitives::{EthEventPayload, MessagePayload};
use serde_wasm_bindgen::from_value;
use tracing::{trace, warn};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

pub(crate) fn setup_listeners() {
    // Runtime `on_message` event
    let closure = Closure::wrap(Box::new(move |payload: JsValue, sender: JsValue| {
        trace!("runtime::on_message_add_listener: {:?}", payload);
        spawn_local(async move {
            let ext_rc = get_extension();
            ext_rc
                .borrow()
                .runtime_on_message(payload, sender)
                .await;
        });
    }) as Box<dyn FnMut(JsValue, JsValue)>);
    runtime::on_message_add_listener(closure.as_ref().unchecked_ref());
    closure.forget();

    // Runtime `on_connect` event
    let closure = Closure::wrap(Box::new(move |port: JsValue| {
        trace!("runtime::on_connect_add_listener: {:?}", port);
        spawn_local(async move {
            let ext_rc = get_extension();
            ext_rc.borrow().runtime_on_connect(port).await;
        });
    }) as Box<dyn FnMut(JsValue)>);
    runtime::on_connect_add_listener(closure.as_ref().unchecked_ref());
    closure.forget();

    // Idle `on_state_changed` event
    let closure = Closure::wrap(Box::new(move |state: JsValue| {
        trace!("idle::on_state_changed_add_listener: {:?}", state);
        spawn_local(async move {
            let ext_rc = get_extension();
            ext_rc.borrow_mut().idle_on_state_changed(state).await;
        });
    }) as Box<dyn FnMut(JsValue)>);
    idle::on_state_changed_add_listener(closure.as_ref().unchecked_ref());
    closure.forget();

    // Tabs `on_updated` event
    let closure = Closure::wrap(Box::new(
        move |tab_id: JsValue, change_info: JsValue, tab: JsValue| {
            trace!(
                "Tab updated: tab_id={:?}, change_info={:?}, tab={:?}",
                tab_id,
                change_info,
                tab
            );
            let ext = get_extension();
            ext.borrow().tabs_on_updated(tab_id, change_info);
        },
    ) as Box<dyn FnMut(JsValue, JsValue, JsValue)>);
    tabs::on_updated_add_listener(closure.as_ref().unchecked_ref());
    closure.forget();

    // Tabs `on_activated` event
    let closure = Closure::wrap(Box::new(move |active_info: JsValue| {
        trace!("tabs::on_activated_add_listener: {:?}", active_info);
        spawn_local(async move {
            let ext_rc = get_extension();
            ext_rc.borrow().tabs_on_activated(active_info).await;
        });
    }) as Box<dyn FnMut(JsValue)>);
    tabs::on_activated_add_listener(closure.as_ref().unchecked_ref());
    closure.forget();

    // Alarms `on_alarm` event
    let closure = Closure::wrap(Box::new(move |alarm: JsValue| {
        trace!("alarms::on_alarm_add_listener: {:?}", alarm);
        let ext_rc = get_extension();
        ext_rc.borrow().alarms_on_alarm(alarm);
    }) as Box<dyn FnMut(JsValue)>);
    alarms::on_alarm_add_listener(closure.as_ref().unchecked_ref());
    closure.forget();
}

// Send an event to a specific tab
async fn send_event_to_tab(tab_id: u32, event: String, args: JsValue) -> Result<(), JsValue> {
    let event = MessagePayload::EthEvent(EthEventPayload::new(event, args));
    tabs::send_message_to_tab(tab_id, event.to_js_value())
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
    selector: tabs::Query,
) {
    // Query tabs based on the provided selector
    let tabs_js = match tabs::query(selector).await {
        Ok(tabs) => tabs,
        Err(e) => {
            warn!("Failed to query tabs: {:?}", e);
            return;
        }
    };

    // Convert to Vec<tabs::Info> and default to an empty array on error
    let tabs: Vec<tabs::Info> = from_value(tabs_js).unwrap_or_default();
    let args_js = args.unwrap_or_else(JsValue::undefined);

    trace!(event, tab_count = tabs.len(), "Sending event to tabs");

    // Filter tabs with valid `id` and `url`, then send the event to each
    spawn_local(async move {
        for tab in tabs
            .iter()
            .filter(|tab| tab.valid())
        {
            let tab_id = tab.id.expect("Tab id should exist after filtering");

            if let Err(e) = send_event_to_tab(tab_id, event.to_owned(), args_js.clone()).await {
                warn!(tab_id, error = ?e, "Failed to send event to tab");
            } else {
                trace!(tab_id, "Event sent successfully to tab");
            }
        }
    });
}
