use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter},
};

use chrome_sys::tabs::{self, send_message_to_tab};
use gloo_utils::format::JsValueSerdeExt;
use serde_json::json;
use wasm_bindgen::JsValue;

// pub async fn get_local_setting_on_tab<T: DeserializeOwned>(
//     tab: &tabs::Info,
//     key: &str,
// ) -> Option<T> {
//     let func = r#"
//         (key) => {
//             try {
//                 return localStorage.getItem(key);
//             } catch (e) {
//                 console.error("Error accessing localStorage:", e);
//                 return null;
//             }
//         }
//     "#;

//     // Call `execute_script` with the function and key as an argument
//     let result = execute_script(&tab, func, vec![JsValue::from_str(key)])
//         .await
//         .ok()?;

//     // Process the `InjectionResult` and deserialize the JSON string into `T`
//     result
//         .get(0)?
//         .result
//         .as_ref()
//         .and_then(|json_str| serde_json::from_str(json_str).ok())
// }

// pub async fn set_local_setting(tab: &tabs::Info, key: &str, val: &str) -> Result<(), JsValue> {
//     let func = "(key, val) => { localStorage.setItem(key, val); window.location.reload(); }";
//     let args = vec![JsValue::from_str(key), JsValue::from_str(val)];

//     execute_script(&tab, func, args).await.map(|_| ())
// }

// pub fn get_local_setting<T: DeserializeOwned>(key: &str) -> Option<T> {
//     let storage = window()?.local_storage().ok()??;
//     let result = storage.get_item(key).ok()??;
//     serde_json::from_str(&result).ok()
// }

// pub async fn toggle_local_setting(key: &str) {
//     if let Some(tab) = get_active_tab().await {
//         let current_value: Option<bool> = get_local_setting_on_tab(&tab, key).await;
//         let new_value = !current_value.unwrap_or(false);
//         set_local_setting(&tab, key, &serde_json::to_string(&new_value).unwrap()).await;
//         window().unwrap().close();
//     }
// }

// pub async fn get_initial_settings(tab: &Option<tabs::Info>) -> Vec<bool> {
//     if let Some(tab) = tab {
//         vec![get_local_setting_on_tab::<bool>(&tab, "APPEAR_AS_MM")
//             .await
//             .unwrap_or(false)]
//     } else {
//         vec![false]
//     }
// }

pub async fn update_current_chain(tab: &Option<tabs::Info>) {
    if let Some(tab) = tab {
        let msg = JsValue::from_serde(&json!({
            "type": "embedded:action",
            "action": { "type": "getCurrentChain" }
        }))
        .unwrap();
        send_message_to_tab(tab, msg)
            .await
            .inspect_err(|e| tracing::error!(?e, "failed to send message to tab"))
            .ok();
    }
}

// pub fn is_injected_url(url: &str) -> bool {
//     url.starts_with("http") || url.starts_with("file")
// }

// Enum to allow passing either a String or a HashMap for custom styling
#[derive(Clone)]
pub enum StringOrMap {
    String(String),
    Map(HashMap<String, String>),
}

impl From<HashMap<String, String>> for StringOrMap {
    fn from(map: HashMap<String, String>) -> Self {
        StringOrMap::Map(map)
    }
}

impl From<String> for StringOrMap {
    fn from(string: String) -> Self {
        StringOrMap::String(string)
    }
}

impl From<&str> for StringOrMap {
    fn from(string: &str) -> Self {
        StringOrMap::String(string.to_string())
    }
}

impl Display for StringOrMap {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            StringOrMap::String(s) => write!(f, "{s}"),
            StringOrMap::Map(map) => {
                let style = map
                    .iter()
                    .fold(String::new(), |acc, (k, v)| format!("{acc}{k}: {v}; "));
                write!(f, "{style}")
            }
        }
    }
}

impl From<StringOrMap> for String {
    fn from(custom_style: StringOrMap) -> Self {
        match custom_style {
            StringOrMap::String(s) => s,
            StringOrMap::Map(map) => map
                .iter()
                .fold(String::new(), |acc, (k, v)| format!("{acc}{k}: {v}; ")),
        }
    }
}
