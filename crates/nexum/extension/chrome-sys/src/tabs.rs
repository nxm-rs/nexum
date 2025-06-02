use gloo_utils::format::JsValueSerdeExt;
use js_sys::{Function, Promise};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_wasm_bindgen::{from_value, to_value};
use tracing::trace;
use url::Url;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Query {
    pub status: Option<String>,
    pub last_focused_window: Option<bool>,
    pub window_id: Option<i32>,
    pub window_type: Option<String>,
    pub active: Option<bool>,
    pub index: Option<i32>,
    pub title: Option<String>,
    pub url: Option<Value>,
    pub current_window: Option<bool>,
    pub highlighted: Option<bool>,
    pub discarded: Option<bool>,
    pub auto_discardable: Option<bool>,
    pub pinned: Option<bool>,
    pub audible: Option<bool>,
    pub muted: Option<bool>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Info {
    pub id: Option<u32>,
    pub url: Option<String>,
}

impl Info {
    /// Given the tab information, confirm via the URL if it is a valid
    /// tab that can be used by the extension.
    pub fn valid(&self) -> bool {
        let id_and_url_exist = self.id.is_some() && self.url.is_some();
        let url_is_http_or_file = self
            .url
            .as_ref()
            .is_some_and(|url| match Url::parse(url) {
                Ok(parsed_url) => {
                    trace!("Parsed URL: {:?}", parsed_url);
                    ["http", "https", "file"].contains(&parsed_url.scheme())
                }
                Err(e) => {
                    trace!("Failed to parse URL: {:?}, Error: {:?}", url, e);
                    false
                }
            });

        // Both conditions must be true for `valid` to return true
        id_and_url_exist && url_is_http_or_file
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveInfo {
    pub tab_id: u32,
    pub window_id: u32,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeInfo {
    pub url: Option<String>,
}

#[wasm_bindgen]
extern "C" {
    // Binding for chrome.tabs.sendMessage
    #[wasm_bindgen(js_namespace = ["chrome", "tabs"], js_name = sendMessage, catch)]
    fn send_message_to_tab_js(tab_id: u32, message: JsValue) -> Result<Promise, JsValue>;

    // Binding for chrome.tabs.query
    #[wasm_bindgen(js_namespace = ["chrome", "tabs"], js_name = query)]
    fn query_js(query_info: &JsValue, callback: &Function);

    // Binding for chrome.tabs.get
    #[wasm_bindgen(js_namespace = ["chrome", "tabs"], js_name = get)]
    fn get_tabs_js(tab_id: u32) -> Promise;

    // Binding for chrome.tabs.onRemoved.addListener
    #[wasm_bindgen(js_namespace = ["chrome", "tabs", "onRemoved"], js_name = addListener)]
    pub fn add_tab_removed_listener(callback: &Function);

    // Binding for chrome.tabs.onUpdated.addListener
    #[wasm_bindgen(js_namespace = ["chrome", "tabs", "onUpdated"], js_name = addListener)]
    pub fn add_tab_updated_listener(callback: &Function);

    // Binding for chrome.tabs.onActivated.addListener
    #[wasm_bindgen(js_namespace = ["chrome", "tabs", "onActivated"], js_name = addListener)]
    pub fn add_tab_activated_listener(callback: &Function);
}

// Rust wrappers

// Wrapper for send_message_to_tab, directly taking a JsValue message
pub async fn send_message_to_tab(tab: &Info, message: JsValue) -> Result<JsValue, JsValue> {
    JsFuture::from(send_message_to_tab_js(tab.id.unwrap(), message)?).await
}

// Function to query tabs with QueryInfo struct
pub async fn query(query_info: Query) -> Result<JsValue, JsValue> {
    // Serialize QueryInfo to JsValue
    let query_info_js = to_value(&query_info).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let (sender, receiver) = futures::channel::oneshot::channel();
    let callback = Closure::once_into_js(move |response: JsValue| {
        let _ = sender.send(response);
    });
    query_js(&query_info_js, callback.unchecked_ref());
    receiver
        .await
        .map_err(|_| JsValue::from_str("Failed to receive response"))
}

pub async fn get_active_tab() -> Option<Info> {
    let query_info = Query {
        active: Some(true),
        current_window: Some(true),
        ..Default::default()
    };
    let response = query(query_info).await.ok()?;
    let tabs: Vec<Info> = response.into_serde().ok()?;

    tabs.first().cloned()
}

pub async fn get(tab_id: u32) -> Result<Info, JsValue> {
    JsFuture::from(get_tabs_js(tab_id))
        .await
        .map(|response| -> Info {
            from_value(response)
                .map_err(|_| JsValue::from_str("Failed to parse response"))
                .unwrap()
        })
        .map_err(|_| JsValue::from_str("Failed to receive response"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_http_url() {
        let info = Info {
            id: Some(1),
            url: Some("http://example.com".to_string()),
        };
        assert!(info.valid());
    }

    #[test]
    fn test_valid_file_url() {
        let info = Info {
            id: Some(2),
            url: Some("file:///path/to/file".to_string()),
        };
        assert!(info.valid());
    }

    #[test]
    fn test_invalid_about_url() {
        let info = Info {
            id: Some(3),
            url: Some("about:blank".to_string()),
        };
        assert!(!info.valid());
    }

    #[test]
    fn test_invalid_no_id() {
        let info = Info {
            id: None,
            url: Some("http://example.com".to_string()),
        };
        assert!(!info.valid());
    }

    #[test]
    fn test_invalid_no_url() {
        let info = Info {
            id: Some(4),
            url: None,
        };
        assert!(!info.valid());
    }

    #[test]
    fn test_invalid_scheme() {
        let info = Info {
            id: Some(5),
            url: Some("ftp://example.com".to_string()),
        };
        assert!(!info.valid());
    }
}
