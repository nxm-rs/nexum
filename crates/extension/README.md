# extension

## `browser-ui`

## `chrome-sys`

FFI bindings for the chrome extension APIs.

## `injected`

Injected initializes provider with `initialize_provider` which defines or overwrites `window.ethereum` object with `EthereumProvider` object.

`EthereumProvider` object initialization does the following:
1. Registers a `window.addEventListener` listener that listens to responses from the extension and sends the response to relevant request callback.
2. Registers a `eip6963:requestProvider` event listener that announces the provider as 6963 provider.

`Provider.request` method sends the request to the extension and waits for a response from the extension which is then returned in the promise.

## `injector`

`injector` is a content script that does the following:

1. Registers a [`chrome.runtime.onMessage`](https://developer.chrome.com/docs/extensions/reference/api/runtime#event-onMessage) listener that forwards all the `ProtocolMessage` messages from the extension to the page using [`window.postMessage`](https://developer.mozilla.org/en-US/docs/Web/API/Window/postMessage).
2. Registers a [`window.addEventListener`] listener that listens for messages from the page and forwards it to the extension using [`chrome.runtime.sendMessage`](https://developer.chrome.com/docs/extensions/reference/api/runtime#method-sendMessage)
3. Injects the `injected` wasm module into the page.

## `worker`

`worker` is a background script that does the following:

1. registers event listeners:
    1. `runtime`
        1. `chrome.runtime.onMessage` -- proxies the requests to the nexum rpc server and returns responses back to relevant pages.
        1. `chrome.runtime.onConnect` -- called by the browser-ui when connected
    1. `tabs` - manages a mapping of tabId to tab origin
        1. `chrome.tabs.onUpdated`
        1. `chrome.tabs.onActivated`
        1. `chrome.tabs.onRemoved`
    1. `idle`
        1. `chrome.idle.onStateChanged` -- resets the provider if the state is changed to active
    1. `alarms`
        1. `chrome.alarms.onAlarm` -- performs the connection health check every 30 seconds. TODO: doesn't actually do a connection retry in case of failures??

### TODO

1. What is `frame_summon`
2. What is `embedded_action_res`
