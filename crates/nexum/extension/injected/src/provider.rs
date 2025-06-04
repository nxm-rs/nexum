use async_lock::Mutex;
use futures::future::Either;
use gloo_timers::future::TimeoutFuture;
use gloo_utils::format::JsValueSerdeExt;
use js_sys::Function;
use nexum_primitives::{
    Error, MessageType, ProtocolMessage, Request, RequestWithId, ResponseWithId,
};
use serde_wasm_bindgen::from_value;
use std::{cell::RefCell, collections::HashMap, rc::Rc, sync::Arc, time::Duration};
use tracing::trace;
use uuid::Uuid;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;
use web_sys::{window, CustomEvent, CustomEventInit, MessageEvent};

use crate::eip6963::{EIP6963Provider, EIP6963ProviderDetail, EIP6963ProviderInfo};

// Global thread-local storage for the provider instance
thread_local! {
    static PROVIDER_INSTANCE: RefCell<Option<Rc<EthereumProvider>>> = const { RefCell::new(None) };
}

#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct EthereumProvider {
    #[allow(dead_code)]
    connected: bool,
    #[allow(dead_code)]
    chain_id: String,
    #[allow(dead_code)]
    accounts: Vec<String>,

    // EIP-1193 fields
    event_listeners: RefCell<HashMap<String, Vec<Function>>>,
    pending_requests: Arc<Mutex<HashMap<String, futures::channel::oneshot::Sender<JsValue>>>>,
}

#[wasm_bindgen]
impl EthereumProvider {
    // Constructor for EthereumProvider
    #[wasm_bindgen(constructor)]
    pub fn new() -> EthereumProvider {
        let provider = EthereumProvider {
            connected: false,
            chain_id: "".to_string(),
            accounts: vec![],
            event_listeners: RefCell::new(HashMap::new()),
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
        };

        let provider_rc = Rc::new(provider);
        PROVIDER_INSTANCE.with(|instance| {
            *instance.borrow_mut() = Some(Rc::clone(&provider_rc));
        });

        // Set up internal message handler and request listener
        provider_rc.setup_message_handler();
        EthereumProvider::listen_for_request_provider(Rc::clone(&provider_rc));

        (*provider_rc).clone()
    }

    // Public EIP-1193 `request` method, returns a Promise
    // https://eips.ethereum.org/EIPS/eip-1193#request
    #[wasm_bindgen]
    pub fn request(&self, args: JsValue) -> js_sys::Promise {
        let this = self.clone();
        future_to_promise(async move { this.request_async(args).await })
    }

    // Public method to add an event listener
    pub fn on(&self, event: &str, listener: Function) {
        trace!("Adding listener for event: {}", event);
        let mut listeners = self.event_listeners.borrow_mut();
        listeners
            .entry(event.to_string())
            .or_default()
            .push(listener);
    }

    // Public method to remove an event listener
    pub fn remove_listener(&self, event: &str, listener: &Function) {
        trace!("Removing listener for event: {}", event);
        let mut listeners = self.event_listeners.borrow_mut();
        if let Some(event_listeners) = listeners.get_mut(event) {
            event_listeners.retain(|l| l != listener);

            if event_listeners.is_empty() {
                listeners.remove(event);
            }
        }
    }
}

// Internal methods for EthereumProvider
impl EthereumProvider {
    // Async request handler
    async fn request_async(self, args: JsValue) -> Result<JsValue, JsValue> {
        trace!("Received request: {:?}", args);
        let window = match window() {
            Some(win) => win,
            None => return Err(JsValue::from_str("Window object is unavailable")),
        };

        // Deserialize the incoming request
        let request = match args.into_serde::<Request>() {
            Ok(req) => req,
            Err(_) => return Err(JsValue::from_str("Failed to deserialize Request")),
        };

        // Generate a unique ID for the request
        let id = Uuid::new_v4().to_string();

        // Create a one-shot channel to receive the response
        let (tx, rx) = futures::channel::oneshot::channel::<JsValue>();

        // Store the sender in pending_requests
        self.pending_requests.lock().await.insert(id.clone(), tx);

        // Prepare the payload with the request_id
        let payload = ProtocolMessage::new(MessageType::Request(RequestWithId {
            id: id.clone(),
            request,
        }));

        // Dispatch the request
        if window
            .post_message(
                &JsValue::from(payload),
                window.location().origin().unwrap_or_default().as_str(),
            )
            .is_err()
        {
            return Err(JsValue::from_str("Failed to post message"));
        }

        // Set a timeout duration
        let timeout_duration = Duration::from_secs(30);
        let response = match futures::future::select(
            rx,
            TimeoutFuture::new(timeout_duration.as_millis() as u32),
        )
        .await
        {
            Either::Left((Ok(val), _)) => val,
            Either::Left((Err(_), _)) => JsValue::from_str("Receiver dropped"),
            Either::Right((_, _)) => {
                // Remove the pending request if it timed out
                self.pending_requests.lock().await.remove(&id);
                return Err(JsValue::from_str("Request timed out"));
            }
        };

        trace!("Received response: {:?}", response);

        // Process the response to extract the inner value
        match response.into_serde::<ProtocolMessage>() {
            Ok(protocol_message) => match protocol_message.message {
                MessageType::Response(ResponseWithId { result, .. }) => match result {
                    Ok(result) => Ok(JsValue::from_serde(&result).unwrap_or_else(|_| {
                        JsValue::from_str("Failed to serialize success result")
                    })),
                    Err(Error { message, .. }) => Err(JsValue::from_str(&message)),
                },
                _ => Err(JsValue::from_str("Unexpected message type")),
            },
            Err(_) => Err(JsValue::from_str("Failed to deserialize ProtocolMessage")),
        }
    }

    // EIP-6963: Respond to `requestProvider` event with provider details
    fn listen_for_request_provider(provider_ref: Rc<EthereumProvider>) {
        let callback = Closure::wrap(Box::new(move || {
            EthereumProvider::announce_provider(Rc::clone(&provider_ref));
        }) as Box<dyn FnMut()>);

        window()
            .expect("should have window")
            .add_event_listener_with_callback(
                "eip6963:requestProvider",
                callback.as_ref().unchecked_ref(),
            )
            .expect("could not add requestProvider event listener");

        callback.forget();
    }

    // EIP-6963: Emit the `announceProvider` `CustomEvent` with provider details
    fn announce_provider(provider_ref: Rc<EthereumProvider>) {
        let provider = &*provider_ref;
        let info = provider.get_info();

        // Create an EIP6963ProviderDetail with serialized info and provider as JsValue
        let detail = EIP6963ProviderDetail::new(
            JsValue::from_serde(&info).expect("Failed to serialize provider info"),
            JsValue::from(provider.clone()),
        );

        let event_init = CustomEventInit::new();
        event_init.set_detail(&JsValue::from(detail));

        let custom_event =
            CustomEvent::new_with_event_init_dict("eip6963:announceProvider", &event_init)
                .expect("could not create announceProvider event");

        window()
            .expect("should have window")
            .dispatch_event(&custom_event)
            .expect("could not dispatch announceProvider event");
    }

    // Setup a message handler for handling incoming messages
    fn setup_message_handler(&self) {
        let pending_requests = Arc::clone(&self.pending_requests);
        let closure = Closure::wrap(Box::new(move |event: MessageEvent| {
            trace!("Received message event: {:?}", event);
            let data = event.data();
            trace!("Received message data: {:?}", data);

            // Now, we suspect that data is actually a ProtocolMessage, so if it is a ProtocolMessage, we will look up
            // any pending requests and send the response back through the oneshot channel.
            if let Ok(msg) = from_value::<ProtocolMessage>(data.clone()).map(|m| m.message) {
                trace!("Payload is a ProtocolMessage: {:?}", msg);
                if let MessageType::Response(response_with_id) = msg {
                    trace!("Received response: {:?}", response_with_id);

                    let pending_requests = pending_requests.clone();
                    wasm_bindgen_futures::spawn_local(async move {
                        let mut pending = pending_requests.lock().await;
                        if let Some(sender) = pending.remove(&response_with_id.id) {
                            // Send the determined response back through the oneshot channel
                            if sender.send(data).is_err() {
                                trace!("Failed to send response, receiver dropped");
                            }
                        } else {
                            trace!("No pending request found for id {}", response_with_id.id);
                        }
                    });
                }
            } else {
                trace!("Payload is not a ProtocolMessage.");
            }
        }) as Box<dyn FnMut(_)>);

        window()
            .expect("should have window")
            .add_event_listener_with_callback("message", closure.as_ref().unchecked_ref())
            .expect("could not add event listener");

        closure.forget(); // Prevent the closure from being dropped prematurely
    }
}

// Implementing the `EIP6963Provider` trait for `EthereumProvider`
impl EIP6963Provider for EthereumProvider {
    fn get_info(&self) -> EIP6963ProviderInfo {
        EIP6963ProviderInfo {
            uuid: "00000000-0000-0000-0000-000000000000".to_string(),
            name: "Nexum".to_string(),
            icon: env!("ICON_SVG_BASE64").to_string(),
            rdns: "rs.nexum".to_string(),
        }
    }
}

impl std::default::Default for EthereumProvider {
    fn default() -> Self {
        Self::new()
    }
}
