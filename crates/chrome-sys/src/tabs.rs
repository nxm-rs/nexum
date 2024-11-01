use js_sys::Function;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_wasm_bindgen::{from_value, to_value};
use tracing::{error, trace};
use url::Url;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Query {
    pub status: Option<String>, // "loading" | "complete"
    pub last_focused_window: Option<bool>,
    pub window_id: Option<i32>, // i32 for window ID or windows.WINDOW_ID_CURRENT
    pub window_type: Option<String>, // "normal" | "popup" | "panel" | "app" | "devtools"
    pub active: Option<bool>,
    pub index: Option<i32>,
    pub title: Option<String>,
    pub url: Option<Value>, // String or array of strings
    pub current_window: Option<bool>,
    pub highlighted: Option<bool>,
    pub discarded: Option<bool>,
    pub auto_discardable: Option<bool>,
    pub pinned: Option<bool>,
    pub audible: Option<bool>,
    pub muted: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Info {
    pub id: Option<u32>,
    pub url: Option<String>,
}

impl Info {
    pub fn valid(&self) -> bool {
        // Check both `id` and `url` fields are present
        let id_and_url_exist = self.id.is_some() && self.url.is_some();

        // Check the `url` scheme is either "http" or "file"
        let url_is_http_or_file = self
            .url
            .as_ref()
            .map_or(false, |url| match Url::parse(url) {
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
    #[wasm_bindgen(js_namespace = ["chrome", "tabs"], js_name = sendMessage)]
    fn sendMessageToTab(tab_id: u32, message: JsValue) -> js_sys::Promise;
    // fn sendMessageToTab(tab_id: u32, message: JsValue) -> JsValue;

    // Binding for chrome.tabs.query
    #[wasm_bindgen(js_namespace = ["chrome", "tabs"], js_name = query)]
    fn queryTabs(query_info: &JsValue, callback: &Function);

    // Binding for chrome.tabs.get
    #[wasm_bindgen(js_namespace = ["chrome", "tabs"], js_name = get)]
    fn getTab(tab_id: u32, callback: &Function);

    // Binding for chrome.tabs.onRemoved.addListener
    #[wasm_bindgen(js_namespace = ["chrome", "tabs", "onRemoved"], js_name = addListener)]
    fn addTabRemovedListener(callback: &Function);

    // Binding for chrome.tabs.onUpdated.addListener
    #[wasm_bindgen(js_namespace = ["chrome", "tabs", "onUpdated"], js_name = addListener)]
    fn addTabUpdatedListener(callback: &Function);

    // Binding for chrome.tabs.onActivated.addListener
    #[wasm_bindgen(js_namespace = ["chrome", "tabs", "onActivated"], js_name = addListener)]
    fn addTabActivatedListener(callback: &Function);
}

// Rust wrappers

// Wrapper for send_message_to_tab, directly taking a JsValue message
pub async fn send_message_to_tab(tab_id: u32, message: JsValue) -> Result<JsValue, JsValue> {
    trace!("Sending message to tab {}", tab_id);
    trace!("Message: {:?}", message);

    // Call sendMessageToTab, which returns a Promise
    let promise = sendMessageToTab(tab_id, message);

    // Convert the Promise to a JsFuture and await its result
    let result = JsFuture::from(promise).await;

    // Match on the result, handling success and error cases
    match result {
        Ok(response) => Ok(response),
        Err(e) => {
            error!("Error from sendMessageToTab: {:?}", e);
            Err(e)
        }
    }
}

// Function to query tabs with QueryInfo struct
pub async fn query(query_info: Query) -> Result<JsValue, JsValue> {
    // Serialize QueryInfo to JsValue
    let query_info_js = to_value(&query_info).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let (sender, receiver) = futures::channel::oneshot::channel();
    let callback = Closure::once_into_js(move |response: JsValue| {
        let _ = sender.send(response);
    });
    queryTabs(&query_info_js, callback.unchecked_ref());
    receiver
        .await
        .map_err(|_| JsValue::from_str("Failed to receive response"))
}

pub async fn get(tab_id: u32) -> Result<Info, JsValue> {
    let (sender, receiver) = futures::channel::oneshot::channel();
    let callback = Closure::once_into_js(move |response: JsValue| {
        let _ = sender.send(response);
    });
    getTab(tab_id, callback.unchecked_ref());
    receiver
        .await
        .map(|response| -> Info {
            from_value(response)
                .map_err(|_| JsValue::from_str("Failed to parse response"))
                .unwrap()
        })
        .map_err(|_| JsValue::from_str("Failed to receive response"))
}

// Wrapper for addTabRemovedListener
pub fn on_removed_add_listener(callback: &Function) -> Result<(), JsValue> {
    Ok(addTabRemovedListener(callback))
}

// Wrapper for addTabUpdatedListener without generics
pub fn on_updated_add_listener(callback: &Function) -> Result<(), JsValue> {
    Ok(addTabUpdatedListener(callback))
}

// Wrapper for addTabActivatedListener without generics
pub fn on_activated_add_listener(callback: &Function) -> Result<(), JsValue> {
    Ok(addTabActivatedListener(callback))
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
