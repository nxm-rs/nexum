use std::{cell::RefCell, rc::Rc};

use chrome_sys::alarms;
use serde_wasm_bindgen::from_value;
use tracing::warn;
use wasm_bindgen::JsValue;

use crate::{Extension, CLIENT_STATUS_ALARM_KEY};

// To be used with the `chrome.alarms.onAlarm` event
pub fn alarms_on_alarm(extension: Rc<RefCell<Extension>>, alarm: JsValue) {
    let alarm: alarms::AlarmInfo = from_value(alarm).unwrap();

    if alarm.name == CLIENT_STATUS_ALARM_KEY {
        warn!("Not implemented: should continually check RPC client status");
    }
}
