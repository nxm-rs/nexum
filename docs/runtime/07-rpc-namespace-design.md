# RPC Namespace Design: Generic JSON-RPC Passthrough

## Problem Statement

An earlier design of the `blockchain` interface defined individual functions for each Ethereum RPC method:

```wit
interface blockchain {
    eth-call: func(chain-id: chain-id, to: list<u8>, data: list<u8>) -> result<list<u8>, string>;
    eth-get-logs: func(filter: log-filter) -> result<list<log-entry>, string>;
    eth-block-number: func(chain-id: chain-id) -> result<u64, string>;
}
```

This creates several problems:

1. **Boilerplate multiplication.** Every new `eth_` method requires changes in three places: WIT definition, host trait implementation, and SDK wrapper. The Ethereum JSON-RPC namespace has 30+ methods; most modules will need more than the three currently exposed.

2. **Alloy incompatibility.** Module authors using Rust cannot use alloy's `Provider` API — which provides 80+ typed convenience methods — because the transport layer is locked behind per-method WIT functions. They're forced to manually ABI-encode calldata, call `blockchain::eth_call`, and ABI-decode the result for every interaction.

3. **Namespace rigidity.** Adding new host namespaces (debug_, trace_, etc.) would duplicate the same per-method pattern, compounding the boilerplate further.

The goal: **one WIT function to rule the entire `eth_` namespace**, with a guest-side SDK that gives module authors the full alloy `Provider` API — no manual ABI wrangling, no WIT changes when new methods are needed.

## Design: Generic JSON-RPC Passthrough

### Core Insight

alloy's `Transport` trait is a Tower `Service<RequestPacket, Response = ResponsePacket>`. If we expose a single JSON-RPC dispatch function in WIT, the SDK can implement `Transport` on top of it. This gives guest modules the entire alloy `Provider` API for free — every current and future `eth_` method works automatically.

From the guest's perspective, host function calls are synchronous (they block until the host returns). The returned future resolves in a single poll. This means alloy's async `Provider` methods work with a trivial executor — no real async machinery needed.

### Architecture

```mermaid
flowchart TD
    A["Module author code
    provider.get_block_number()
    provider.call(tx).latest()
    provider.get_logs(&filter)"] -->|full alloy Provider API| B

    B["HostTransport (SDK)
    implements alloy Transport trait"] -->|"csn::request(chain_id, &quot;eth_blockNumber&quot;, &quot;[]&quot;)"| C

    C["WIT boundary
    single generic function"] --> D

    D["Host csn::request impl
    forwards to alloy provider"] -->|"provider.raw_request_dyn(method, params)"| E

    E["Alloy provider stack
    timeout -> retry -> rate-limit -> fallback -> RPC"]
```

## Updated WIT Interface

Replace the `blockchain` interface with `csn`:

```wit
package web3:runtime@0.1.0;

interface csn {
    use types.{chain-id};

    /// JSON-RPC error returned by the provider or the host.
    record json-rpc-error {
        code: s64,
        message: string,
        data: option<string>,
    }

    /// Execute a JSON-RPC request against the specified chain.
    ///
    /// The host forwards the request to the configured alloy provider for the
    /// given chain, applying timeout/retry/rate-limit/fallback middleware
    /// transparently. The method string should include the namespace prefix
    /// (e.g. "eth_call", "eth_getBlockByNumber").
    ///
    /// `params` and the success return value are JSON-encoded strings matching
    /// the JSON-RPC specification. The host handles id/jsonrpc framing; the
    /// guest only provides method + params and receives the `result` field.
    request: func(chain-id: chain-id, method: string, params: string)
        -> result<string, json-rpc-error>;
}
```

The `types` interface is unchanged. The `local-store`, `remote-store`, `msg`, and `logging` interfaces are unchanged.

The `identity` interface provides cryptographic identity — key management and signing:

```wit
interface identity {
    record identity-error {
        code: u16,
        message: string,
    }

    /// Get available signing accounts (20-byte Ethereum addresses).
    accounts: func() -> result<list<list<u8>>, identity-error>;

    /// Sign raw bytes with the specified account.
    /// Returns a 65-byte ECDSA secp256k1 signature (r ‖ s ‖ v).
    sign: func(account: list<u8>, data: list<u8>) -> result<list<u8>, identity-error>;

    /// Sign EIP-712 typed data with the specified account.
    sign-typed-data: func(account: list<u8>, typed-data: string) -> result<list<u8>, identity-error>;
}
```

The universal `headless-module` world (in `web3:runtime`) contains the platform-agnostic interfaces:

```wit
world headless-module {
    import csn;          // replaces `import blockchain;`
    import identity;     // cryptographic identity (key management, signing)
    import local-store;
    import remote-store;
    import msg;
    import logging;

    export init: func(config: types.config) -> result<_, string>;
    export on-event: func(event: types.event) -> result<_, string>;
}
```

Downstream distributions may define additional worlds that `include` `web3:runtime/headless-module` and import further domain-specific interfaces (see [Downstream extensions: domain namespaces](#downstream-extensions-domain-namespaces) below).

### What This Replaces

| Before (per-method) | After (generic) |
|---|---|
| `blockchain::eth-call(chain-id, to, data)` | `csn::request(chain-id, "eth_call", params_json)` |
| `blockchain::eth-get-logs(filter)` | `csn::request(chain-id, "eth_getLogs", params_json)` |
| `blockchain::eth-block-number(chain-id)` | `csn::request(chain-id, "eth_blockNumber", "[]")` |
| *n/a — not exposed* | `csn::request(chain-id, "eth_getBalance", params_json)` |
| *n/a — not exposed* | `csn::request(chain-id, "eth_getCode", params_json)` |
| *n/a — not exposed* | `csn::request(chain-id, "eth_getStorageAt", params_json)` |
| *n/a — not exposed* | Any `eth_*` method — no WIT change needed |

### Why JSON Strings (Not `list<u8>`)

- The Ethereum JSON-RPC spec is JSON. alloy serialises params to JSON internally. Using `string` means zero intermediate format — the guest produces JSON, the host forwards JSON to alloy's `raw_request_dyn` which accepts `&RawValue` (a JSON string).
- Debuggability: JSON is human-readable in logs and traces.
- The canonical ABI cost of copying a JSON string across the component boundary is negligible relative to the network RTT of an actual RPC call.
- Binary encoding (CBOR, postcard) would require custom (de)serialisation on both sides, defeating the purpose of minimising boilerplate.

## Host Implementation

The host implementation is minimal — one function handles the entire `eth_` namespace:

```rust
use serde_json::value::RawValue;

impl web3::runtime::csn::Host for NexumHostState {
    async fn request(
        &mut self,
        chain_id: u64,
        method: String,
        params: String,
    ) -> wasmtime::Result<Result<String, JsonRpcError>> {
        // 1. Check if this is a signing method that requires identity delegation
        if self.is_signing_method(&method) {
            return self.dispatch_signing(chain_id, &method, &params).await;
        }

        // 2. Method allowlisting for read-only methods
        if !self.is_read_method_allowed(&method) {
            return Ok(Err(JsonRpcError {
                code: -32601,
                message: format!("method not allowed: {method}"),
                data: None,
            }));
        }

        // 3. Resolve the provider for this chain
        let provider = self.provider_for(chain_id).map_err(|e| {
            JsonRpcError {
                code: -32002,
                message: format!("unknown chain: {chain_id}"),
                data: None,
            }
        })?;

        // 4. Parse params as raw JSON and forward to alloy
        let raw_params: Box<RawValue> = RawValue::from_string(params)
            .map_err(|e| wasmtime::Error::msg(format!("invalid JSON params: {e}")))?;

        match provider.raw_request_dyn(method.into(), &raw_params).await {
            Ok(result) => Ok(Ok(result.get().to_string())),
            Err(e) => Ok(Err(e.into())), // TransportError -> JsonRpcError
        }
    }
}
```

That's it. The alloy provider already has the timeout/retry/rate-limit/fallback tower stack configured per chain (see doc 01). Every read-only `eth_*` method automatically inherits that middleware.

### Method Allowlisting

The host maintains two categories of methods: **read-only methods** (always allowed through the RPC passthrough) and **signing methods** (delegated to the `identity` backend).

#### Read-Only Methods (RPC Passthrough)

```rust
impl NexumHostState {
    fn is_read_method_allowed(&self, method: &str) -> bool {
        // Default allowlist: read-only eth_ methods
        matches!(method,
            "eth_blockNumber"
            | "eth_call"
            | "eth_chainId"
            | "eth_estimateGas"
            | "eth_feeHistory"
            | "eth_gasPrice"
            | "eth_maxPriorityFeePerGas"
            | "eth_getBalance"
            | "eth_getBlockByHash"
            | "eth_getBlockByNumber"
            | "eth_getBlockReceipts"
            | "eth_getCode"
            | "eth_getLogs"
            | "eth_getProof"
            | "eth_getStorageAt"
            | "eth_getTransactionByHash"
            | "eth_getTransactionCount"
            | "eth_getTransactionReceipt"
            // net_ methods
            | "net_version"
        )
    }
}
```

This could be made configurable per-module via `nexum.toml`:

```toml
[module.csn]
# Additional methods beyond the default read-only set.
# Use with caution — write methods can have side-effects.
extra_allowed_methods = ["eth_createAccessList"]
```

The allowlist is runtime-enforced (string matching), not compile-time. This is an acceptable trade-off: the Component Model already provides structural sandboxing (modules can only call `csn::request`, not arbitrary network I/O), and the allowlist adds defence-in-depth for method-level granularity.

#### Signing Methods (Identity Delegation)

When a module calls `csn::request` with a signing method, the host does **not** forward the request to the RPC provider. Instead, it delegates to the `identity` backend for signing, then broadcasts the signed result via RPC.

```rust
impl NexumHostState {
    fn is_signing_method(&self, method: &str) -> bool {
        matches!(method,
            "eth_sendTransaction"
            | "eth_accounts"
            | "eth_signTypedData_v4"
            | "personal_sign"
        )
    }
}
```

These methods are deliberately **not** in the read-only allowlist. They follow a completely different code path through the identity backend.

### Identity Delegation Flow

When a module calls a signing method through `csn::request`, the host intercepts it and delegates to the `Identity` trait:

```mermaid
sequenceDiagram
    participant M as Module (guest)
    participant C as CsnHost
    participant I as Identity backend
    participant R as RPC provider

    M->>C: csn::request(1, "eth_sendTransaction", params)
    C->>C: is_signing_method("eth_sendTransaction") → true
    C->>C: Parse transaction from params
    C->>I: sign(account, tx_hash)
    I-->>C: 65-byte signature (r ‖ s ‖ v)
    C->>C: Assemble signed transaction (RLP-encode with signature)
    C->>R: eth_sendRawTransaction(signed_tx)
    R-->>C: tx_hash
    C-->>M: Ok(tx_hash)
```

The key insight: modules never call `eth_sendRawTransaction` directly (it's not in the read-only allowlist). Instead, `eth_sendTransaction` is intercepted by the host, which uses the `identity` backend to sign, then broadcasts the signed transaction itself.

This pattern applies to all signing methods:

| Method | Identity Delegation |
|---|---|
| `eth_accounts` | Returns accounts from `Identity::accounts()` |
| `eth_sendTransaction` | Signs the transaction via `Identity::sign()`, broadcasts via `eth_sendRawTransaction` |
| `eth_signTypedData_v4` | Signs EIP-712 typed data via `Identity::sign_typed_data()` |
| `personal_sign` | Signs the message via `Identity::sign()` (with EIP-191 prefix) |

### Identity Trait and CsnHost

The host's `csn` implementation is generic over an `Identity` trait. This allows different identity backends (hardware wallet, KMS, in-memory test keys, etc.):

```rust
/// Trait for identity backends that provide signing capabilities.
///
/// The host's csn implementation delegates signing methods to this trait.
/// Implementations can back onto hardware wallets, cloud KMS, in-memory
/// test keys, or any other signing infrastructure.
pub trait Identity: Send + Sync {
    /// Get available signing accounts (20-byte Ethereum addresses).
    fn accounts(&self) -> Result<Vec<Vec<u8>>, IdentityError>;

    /// Sign raw bytes with the specified account.
    /// Returns a 65-byte ECDSA secp256k1 signature (r ‖ s ‖ v).
    fn sign(&self, account: &[u8], data: &[u8]) -> Result<Vec<u8>, IdentityError>;

    /// Sign EIP-712 typed data with the specified account.
    fn sign_typed_data(&self, account: &[u8], typed_data: &str) -> Result<Vec<u8>, IdentityError>;
}

/// The host state is generic over the identity backend.
pub struct CsnHost<I: Identity> {
    providers: HashMap<u64, RootProvider>,
    identity: I,
}

impl<I: Identity> web3::runtime::csn::Host for CsnHost<I> {
    async fn request(
        &mut self,
        chain_id: u64,
        method: String,
        params: String,
    ) -> wasmtime::Result<Result<String, JsonRpcError>> {
        if self.is_signing_method(&method) {
            return self.dispatch_signing(chain_id, &method, &params).await;
        }

        if !self.is_read_method_allowed(&method) {
            return Ok(Err(JsonRpcError {
                code: -32601,
                message: format!("method not allowed: {method}"),
                data: None,
            }));
        }

        let provider = self.provider_for(chain_id)?;
        let raw_params: Box<RawValue> = RawValue::from_string(params)
            .map_err(|e| wasmtime::Error::msg(format!("invalid JSON params: {e}")))?;

        match provider.raw_request_dyn(method.into(), &raw_params).await {
            Ok(result) => Ok(Ok(result.get().to_string())),
            Err(e) => Ok(Err(e.into())),
        }
    }
}

impl<I: Identity> CsnHost<I> {
    /// Dispatch signing methods to the identity backend.
    async fn dispatch_signing(
        &self,
        chain_id: u64,
        method: &str,
        params: &str,
    ) -> wasmtime::Result<Result<String, JsonRpcError>> {
        match method {
            "eth_accounts" => {
                let accounts = self.identity.accounts().map_err(|e| JsonRpcError {
                    code: -32000,
                    message: e.message,
                    data: None,
                })?;
                let hex_accounts: Vec<String> = accounts
                    .iter()
                    .map(|a| format!("0x{}", hex::encode(a)))
                    .collect();
                Ok(Ok(serde_json::to_string(&hex_accounts)?))
            }

            "eth_sendTransaction" => {
                let provider = self.provider_for(chain_id)?;
                // Parse the transaction params
                let tx_params: Vec<serde_json::Value> = serde_json::from_str(params)?;
                let tx = &tx_params[0];

                let from = parse_address(tx.get("from"))?;

                // Fill missing fields (nonce, gas, etc.) via the provider
                let filled_tx = self.fill_transaction(provider, tx).await?;

                // Hash the transaction and sign it
                let tx_hash = filled_tx.signing_hash();
                let signature = self.identity.sign(&from, tx_hash.as_ref())
                    .map_err(|e| JsonRpcError {
                        code: -32000,
                        message: e.message,
                        data: None,
                    })?;

                // Assemble signed transaction and broadcast
                let signed_tx = filled_tx.with_signature(&signature);
                let raw_tx = signed_tx.rlp_encode();

                let raw_params = serde_json::to_string(&[format!("0x{}", hex::encode(&raw_tx))])?;
                let raw_params_box: Box<RawValue> = RawValue::from_string(raw_params)?;
                match provider.raw_request_dyn("eth_sendRawTransaction".into(), &raw_params_box).await {
                    Ok(result) => Ok(Ok(result.get().to_string())),
                    Err(e) => Ok(Err(e.into())),
                }
            }

            "eth_signTypedData_v4" => {
                let params_arr: Vec<serde_json::Value> = serde_json::from_str(params)?;
                let account = parse_address(&params_arr[0])?;
                let typed_data = params_arr[1].to_string();

                let signature = self.identity.sign_typed_data(&account, &typed_data)
                    .map_err(|e| JsonRpcError {
                        code: -32000,
                        message: e.message,
                        data: None,
                    })?;
                Ok(Ok(format!("\"0x{}\"", hex::encode(&signature))))
            }

            "personal_sign" => {
                let params_arr: Vec<serde_json::Value> = serde_json::from_str(params)?;
                let data = parse_hex_bytes(&params_arr[0])?;
                let account = parse_address(&params_arr[1])?;

                // EIP-191 prefix
                let prefixed = format!("\x19Ethereum Signed Message:\n{}", data.len());
                let mut msg = prefixed.into_bytes();
                msg.extend_from_slice(&data);
                let hash = keccak256(&msg);

                let signature = self.identity.sign(&account, &hash)
                    .map_err(|e| JsonRpcError {
                        code: -32000,
                        message: e.message,
                        data: None,
                    })?;
                Ok(Ok(format!("\"0x{}\"", hex::encode(&signature))))
            }

            _ => Ok(Err(JsonRpcError {
                code: -32601,
                message: format!("unknown signing method: {method}"),
                data: None,
            })),
        }
    }
}
```

The `CsnHost` also implements `web3::runtime::identity::Host` directly, delegating to the same `Identity` trait so modules can use the identity WIT interface for raw signing:

```rust
impl<I: Identity> web3::runtime::identity::Host for CsnHost<I> {
    fn accounts(&mut self) -> wasmtime::Result<Result<Vec<Vec<u8>>, IdentityError>> {
        Ok(self.identity.accounts())
    }

    fn sign(
        &mut self,
        account: Vec<u8>,
        data: Vec<u8>,
    ) -> wasmtime::Result<Result<Vec<u8>, IdentityError>> {
        Ok(self.identity.sign(&account, &data))
    }

    fn sign_typed_data(
        &mut self,
        account: Vec<u8>,
        typed_data: String,
    ) -> wasmtime::Result<Result<Vec<u8>, IdentityError>> {
        Ok(self.identity.sign_typed_data(&account, &typed_data))
    }
}
```

## Guest SDK: `HostTransport`

The key SDK addition is a `HostTransport` struct that implements alloy's `Transport` trait by routing through the WIT `csn::request` host function.

### Transport Implementation

```rust
use alloy_json_rpc::{
    ErrorPayload, RequestPacket, Response, ResponsePacket, ResponsePayload,
    SerializedRequest,
};
use alloy_transport::{BoxTransport, Transport, TransportError, TransportFut};
use tower::Service;
use std::task::{Context, Poll};

/// An alloy-compatible transport that routes JSON-RPC requests through the
/// nexum host runtime. Synchronous from the guest's perspective — the host
/// function blocks until the RPC response is available.
#[derive(Debug, Clone)]
pub struct HostTransport {
    chain_id: u64,
}

impl HostTransport {
    pub fn new(chain_id: u64) -> Self {
        Self { chain_id }
    }
}

impl Service<RequestPacket> for HostTransport {
    type Response = ResponsePacket;
    type Error = TransportError;
    type Future = TransportFut<'static>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // Always ready — host function calls are synchronous from the guest.
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: RequestPacket) -> Self::Future {
        let chain_id = self.chain_id;
        Box::pin(async move {
            match req {
                RequestPacket::Single(req) => {
                    let resp = dispatch_single(chain_id, &req)?;
                    Ok(ResponsePacket::Single(resp))
                }
                RequestPacket::Batch(reqs) => {
                    let resps: Result<Vec<_>, _> = reqs
                        .iter()
                        .map(|r| dispatch_single(chain_id, r))
                        .collect();
                    Ok(ResponsePacket::Batch(resps?))
                }
            }
        })
    }
}

impl Transport for HostTransport {
    fn boxed(self) -> BoxTransport
    where
        Self: Sized + Clone + Send + Sync + 'static,
    {
        BoxTransport::new(self)
    }
}

/// Dispatch a single JSON-RPC request through the host function.
fn dispatch_single(
    chain_id: u64,
    req: &SerializedRequest,
) -> Result<Response<Box<RawValue>>, TransportError> {
    let method = req.method();
    let params_json = req.params().map(|p| p.get()).unwrap_or("[]");

    // This calls the WIT-imported host function. Synchronous from the guest's
    // perspective — the host executes the RPC call asynchronously and returns
    // the result when ready.
    match csn::request(chain_id, method, params_json) {
        Ok(result_json) => {
            let payload: Box<RawValue> = RawValue::from_string(result_json)
                .map_err(|e| TransportError::deser_err(e, "host response"))?;
            Ok(Response {
                id: req.id().clone(),
                payload: ResponsePayload::Success(payload),
            })
        }
        Err(e) => {
            // Return a JSON-RPC error response rather than a transport error,
            // so alloy can surface the RPC error code/message to the caller.
            Ok(Response {
                id: req.id().clone(),
                payload: ResponsePayload::Failure(ErrorPayload {
                    code: e.code,
                    message: e.message,
                    data: e.data.and_then(|d| RawValue::from_string(d).ok()),
                }),
            })
        }
    }
}
```

### Why This Works Without Real Async

The `call()` method returns a `Box::pin(async move { ... })` — but the body is entirely synchronous. The `csn::request` host function blocks from the guest's perspective (the host runs the actual RPC call asynchronously via wasmtime's `func_wrap_async`, but the guest sees a normal function call that returns a value). The future resolves in a single poll.

This means alloy's `Provider` methods — which `await` the transport internally — complete immediately when driven by any executor. The SDK provides a minimal single-threaded executor:

```rust
/// Drive a future to completion. Since the HostTransport resolves
/// synchronously, this is a single-poll operation — no actual async
/// scheduling occurs.
pub fn block_on<F: Future>(future: F) -> F::Output {
    futures_executor::block_on(future)
}
```

`futures-executor` is no-std-compatible and adds no meaningful overhead.

### Provider Constructor

```rust
use alloy_provider::RootProvider;
use alloy_rpc_client::RpcClient;

/// Create an alloy `Provider` backed by the nexum host runtime.
///
/// The returned provider supports the full alloy `Provider` API — all `eth_*`
/// methods, builder patterns, typed responses — routing every request through
/// the host's RPC stack (timeout, retry, rate-limit, failover).
///
/// ```rust
/// let provider = nexum_sdk::provider(42161);
/// let block = provider.get_block_number().await?;
/// ```
pub fn provider(chain_id: u64) -> RootProvider {
    let transport = HostTransport::new(chain_id);
    let client = RpcClient::new(transport, false); // false = not local
    RootProvider::new(client)
}
```

## Eliminating `block_on`: Async Module Functions

### The Problem

alloy's `Provider` is async. Without help, module authors would need `block_on()` around every RPC call:

```rust
let block_num = block_on(provider.get_block_number())?;  // noisy
let balance = block_on(provider.get_balance(addr).latest())?;  // everywhere
```

This is verbose and obscures the actual logic. But we can't reimplement every `Provider` method as a synchronous wrapper — that defeats the purpose of the generic passthrough.

### The Solution: Named Event Handlers + `async fn`

The proc macro (see doc 05) already generates the WIT export boilerplate. We extend it in two ways:

1. **Named event handlers** — instead of writing the `match event { ... }` dispatch manually, module authors implement `on_block`, `on_logs`, and/or `on_timer`. The macro generates the `on_event` match.
2. **`async fn` support** — handlers can be async. The macro wraps the generated `on_event` in `block_on()`, so `.await` works naturally.
3. **Provider injection** — if a handler accepts `&RootProvider` as a second parameter, the macro creates the provider from the event's chain_id and passes it in.

**What the module author writes:**

```rust
#[nexum::module]
struct MyModule;

impl MyModule {
    async fn on_block(block: BlockData, provider: &RootProvider) -> Result<()> {
        let block_num = provider.get_block_number().await?;       // natural .await
        let balance = provider.get_balance(addr).latest().await?; // no block_on
        Ok(())
    }

    async fn on_logs(logs: Vec<LogEntry>, provider: &RootProvider) -> Result<()> {
        for log in &logs {
            // ...
        }
        Ok(())
    }

    // on_timer not defined -> timer events silently ignored
}
```

**What the macro generates:**

```rust
impl Guest for MyModule {
    fn on_event(event: types::Event) -> Result<(), String> {
        nexum_sdk::block_on(async {
            match event {
                Event::Block(block) => {
                    let provider = nexum_sdk::provider(block.chain_id);
                    MyModule::on_block(block, &provider).await
                }
                Event::Logs(logs) => {
                    let provider = nexum_sdk::provider(logs[0].chain_id);
                    MyModule::on_logs(logs, &provider).await
                }
                Event::Timer(_) => Ok(()),  // no handler defined
            }
        }).map_err(|e| e.to_string())
    }
}
```

The generated code calls `block_on` exactly once — at the top-level export boundary. Inside the async block, all `.await` calls resolve immediately (the `HostTransport` is synchronous under the hood). No real async scheduler runs. No tokio. No waker machinery. It's syntactic sugar that costs nothing at runtime.

### Named Handler Conventions

| Handler | Payload | Optional injectable context |
|---|---|---|
| `on_block(block)` | `BlockData` | `provider: &RootProvider` (from `block.chain_id`) |
| `on_logs(logs)` | `Vec<LogEntry>` | `provider: &RootProvider` (from `logs[0].chain_id`) |
| `on_timer(timestamp)` | `u64` | None (no chain context) |

The macro inspects each handler's signature:
- **Second parameter is `&RootProvider`** -> inject `nexum_sdk::provider(chain_id)`
- **No second parameter** -> pass only the payload
- **Async handlers** -> wrapped in `block_on`; sync handlers called directly
- **Missing handlers** -> `Ok(())` for that variant (no-op)

**Escape hatch:** defining `on_event` directly takes precedence — the macro uses it as-is (wrapping in `block_on` if async) and ignores named handlers.

### Why This Works

1. **WIT exports are synchronous.** The Component Model export signature is `func(event) -> result<_, string>` — no async. The macro bridges this by wrapping the generated dispatch in `block_on`.

2. **The transport resolves in one poll.** `HostTransport::call()` returns a future whose body is entirely synchronous (it calls the WIT host function, which blocks). When alloy's `Provider` awaits the transport, the future completes immediately.

3. **`futures_executor::block_on` is trivial.** It creates a waker, polls the future once, gets `Poll::Ready`. No thread parking, no event loop. On WASM single-threaded targets this is a no-op wrapper.

4. **Composability.** Module authors can use alloy's builder patterns naturally inside any handler:

   ```rust
   async fn on_block(block: BlockData, provider: &RootProvider) -> Result<()> {
       // EthCall builder — .latest() and .await both work
       let result = provider.call(tx).latest().await?;

       // Filter builder — standard alloy ergonomics
       let logs = provider.get_logs(&filter).await?;

       // Raw request for unlisted methods
       let proof: EIP1186AccountProofResponse = provider
           .raw_request("eth_getProof".into(), (addr, keys, "latest"))
           .await?;
       Ok(())
   }
   ```

5. **Sync handlers still work.** Handlers that don't need RPC can be plain `fn`:

   ```rust
   fn on_timer(timestamp: u64) -> Result<()> {
       info!("timer fired at {timestamp}");
       Ok(())
   }
   ```

### Comparison

| Approach | Event dispatch boilerplate | RPC call boilerplate | New methods need shimming? | alloy-native? |
|---|---|---|---|---|
| Manual `on_event` + `block_on()` | `match event { ... }` every module | `block_on(...)` every call | No | Yes |
| **Named handlers + async macro** | **None (generated)** | **None (`.await`)** | **No** | **Yes** |

The named handler + async macro approach eliminates boilerplate at both the event dispatch level and the RPC call level.

## Module Author Experience

### Before (Per-Method WIT)

```rust
use nexum_sdk::prelude::*;
use nexum_sdk::abi::sol;

sol! {
    function balanceOf(address owner) view returns (uint256);
}

#[nexum::module]
struct MyModule;

impl MyModule {
    fn on_event(event: Event) -> Result<()> {
        if let Event::Block(block) = event {
            // Manual ABI encode
            let calldata = balanceOfCall { owner: addr }.abi_encode();

            // Raw host call — returns opaque bytes
            let result_bytes = blockchain::eth_call(
                block.chain_id,
                &token_addr.to_vec(),
                &calldata,
            )?;

            // Manual ABI decode
            let balance = balanceOfCall::abi_decode_returns(&result_bytes)?;

            // Want eth_getBalance? Not available. Want eth_getCode? Not available.
            // Each new method needs WIT + host + SDK changes.
        }
        Ok(())
    }
}
```

### After (Generic RPC + named handlers + provider injection)

```rust
use nexum_sdk::prelude::*;

sol! {
    function balanceOf(address owner) view returns (uint256);
}

#[nexum::module]
struct MyModule;

impl MyModule {
    // Named handler — macro generates the match dispatch + provider injection
    async fn on_block(block: BlockData, provider: &RootProvider) -> Result<()> {
        // Full alloy Provider API — natural .await, provider injected
        let block_num = provider.get_block_number().await?;
        let eth_balance = provider.get_balance(addr).latest().await?;
        let code = provider.get_code_at(contract).latest().await?;

        // Typed contract calls with the EthCall builder
        let tx = TransactionRequest::default()
            .to(token_addr)
            .input(balanceOfCall { owner: addr }.abi_encode().into());

        let result = provider.call(tx).latest().await?;
        let balance = balanceOfCall::abi_decode_returns(&result)?;

        // Log queries with alloy's Filter builder
        let filter = Filter::new()
            .address(contract)
            .event_signature(Transfer::SIGNATURE_HASH)
            .from_block(block.number - 100);
        let logs = provider.get_logs(&filter).await?;

        // Raw request for anything not wrapped by Provider
        let proof: EIP1186AccountProofResponse = provider
            .raw_request("eth_getProof".into(), (addr, keys, "latest"))
            .await?;

        Ok(())
    }

    // Only implement handlers for event types you care about.
    // No on_logs or on_timer -> those events are no-ops.
}
```

Every alloy `Provider` method works. No WIT changes. No host-side per-method code. No `block_on`. No `match event { ... }`. No manual provider construction.

## Downstream extensions: domain namespaces

The generic JSON-RPC passthrough pattern generalises beyond the `eth_` namespace. A downstream distribution that needs to expose a domain-specific service — for example, a REST API or a non-Ethereum JSON-RPC namespace — has two options:

### Option A: Separate Interface (Recommended)

Define a new WIT interface in a downstream package that `include`s `web3:runtime/headless-module`. As a concrete example, the Shepherd distribution exposes the CoW Protocol REST API via a dedicated `cow` interface:

```wit
// Example from the downstream shepherd:cow WIT package (not part of nexum itself)
interface cow {
    use web3:runtime/types.{chain-id};

    record api-error {
        status: u16,
        message: string,
        body: option<string>,
    }

    /// HTTP-style request to the CoW Protocol API.
    ///
    /// The host routes to the correct CoW API base URL for the given chain
    /// (e.g. https://api.cow.fi/mainnet for chain 1, /arbitrum for chain
    /// 42161). The path is relative to the base URL.
    ///
    /// method: "GET" | "POST" | "PUT" | "DELETE"
    /// path: relative API path, e.g. "/api/v1/orders"
    /// body: optional JSON request body
    ///
    /// Returns the response body as a JSON string.
    request: func(
        chain-id: chain-id,
        method: string,
        path: string,
        body: option<string>,
    ) -> result<string, api-error>;
}
```

```wit
world shepherd-module {
    include web3:runtime/headless-module;
    import cow;       // CoW Protocol API access
    import order;     // kept for backwards compat; could merge into cow
}
```

The host implementation is similarly minimal:

```rust
impl shepherd::cow::cow::Host for ShepherdHostState {
    async fn request(
        &mut self,
        chain_id: u64,
        method: String,
        path: String,
        body: Option<String>,
    ) -> wasmtime::Result<Result<String, ApiError>> {
        let base_url = self.cow_api_url_for(chain_id)?;
        let url = format!("{base_url}{path}");

        let req = self.http_client.request(method.parse()?, &url);
        let req = match body {
            Some(b) => req.header("content-type", "application/json").body(b),
            None => req,
        };

        let resp = req.send().await?;
        let status = resp.status().as_u16();

        if status >= 400 {
            let body = resp.text().await.ok();
            return Ok(Err(ApiError { status, message: "request failed".into(), body }));
        }

        Ok(Ok(resp.text().await?))
    }
}
```

### Option B: JSON-RPC Style (Unified)

Route domain-specific methods (e.g. `cow_*`) through the same `csn::request` function:

```rust
// Guest usage:
let order_uid: String = block_on(provider.raw_request(
    "cow_submitOrder".into(),
    serde_json::json!({ "sellToken": "0x...", "buyToken": "0x...", ... }),
))?;
```

The host dispatches by method prefix:

```rust
async fn request(&mut self, chain_id: u64, method: String, params: String)
    -> wasmtime::Result<Result<String, JsonRpcError>>
{
    if method.starts_with("eth_") || method.starts_with("net_") {
        self.dispatch_rpc(chain_id, &method, &params).await
    } else if method.starts_with("cow_") {
        self.dispatch_cow(chain_id, &method, &params).await
    } else {
        Ok(Err(JsonRpcError { code: -32601, message: "unknown namespace".into(), data: None }))
    }
}
```

**Option A is recommended.** When the downstream API is REST (as the CoW Protocol API is), forcing it into JSON-RPC semantics adds a translation layer on both sides. A separate interface in its own WIT package keeps the contract explicit and makes it clear in the WIT world what capabilities a module has. It also allows independent evolution — nexum's `csn` interface doesn't need to know about downstream extensions, and vice versa.

Downstream SDK crates (e.g. `shepherd-sdk`) can then wrap the downstream interface with a typed client (e.g. `CowClient`) in the same way that `nexum-sdk` wraps `csn` with `HostTransport` and `provider()`. Module authors pick the SDK that matches their target world; the underlying host adapter pattern is identical.

## Updated SDK Crate Structure

```
nexum-sdk/
├── Cargo.toml
├── src/
│   ├── lib.rs               # re-exports, prelude, provider() constructor
│   ├── bindings.rs           # generated WIT bindings
│   ├── transport.rs          # HostTransport (alloy Transport impl)
│   ├── local_store.rs        # TypedState helpers (serde over local-store)
│   ├── identity.rs           # IdentityClient (typed identity helpers)
│   ├── abi.rs                # alloy-sol-types integration
│   ├── log.rs                # logging macros
│   ├── error.rs              # error types
│   └── testing.rs            # mock host, test harness
└── macros/
    └── src/
        └── lib.rs            # #[nexum::module] proc macro
```

New dependencies (in `nexum-sdk`):

```toml
[dependencies]
alloy-transport    = { version = "1.5", default-features = false }
alloy-json-rpc     = { version = "1.5", default-features = false }
alloy-rpc-client   = { version = "1.5", default-features = false }
alloy-provider     = { version = "1.5", default-features = false }
alloy-rpc-types    = { version = "1.5", default-features = false }
alloy-primitives   = { version = "1.5", default-features = false }
alloy-sol-types    = { version = "1.5", default-features = false }
futures-executor   = { version = "0.3", default-features = false }
serde              = { version = "1", default-features = false, features = ["derive"] }
serde_json         = { version = "1", default-features = false, features = ["alloc"] }
tower              = { version = "0.5", default-features = false }
```

All alloy crates with `default-features = false` to avoid pulling in reqwest, tokio, or other dependencies that won't compile for `wasm32-wasip2`. The key crates (`alloy-primitives`, `alloy-sol-types`, `alloy-json-rpc`) are already `no_std`-compatible or have WASM-friendly feature flags.

## Updated Prelude

```rust
// nexum_sdk::prelude
pub use crate::bindings::web3::runtime::types::*;
pub use crate::bindings::web3::runtime::csn;
pub use crate::bindings::web3::runtime::identity;
pub use crate::bindings::web3::runtime::local_store;
pub use crate::bindings::web3::runtime::remote_store;
pub use crate::bindings::web3::runtime::msg;
pub use crate::bindings::web3::runtime::logging;
pub use crate::log::{trace, debug, info, warn, error};
pub use crate::local_store::TypedState;
pub use crate::identity::IdentityClient;
pub use crate::transport::HostTransport;
pub use crate::{provider, block_on};
pub use crate::error::{Result, Error};

// Re-export alloy essentials so modules don't need direct alloy dependencies
pub use alloy_primitives::{Address, B256, U256, Bytes};
pub use alloy_sol_types::sol;
pub use alloy_rpc_types::*;
pub use alloy_provider::Provider;
```

## Testing

### MockTransport for Unit Tests

The SDK testing module provides a mock transport that mirrors alloy's own `Asserter`-based testing pattern:

```rust
use nexum_sdk::testing::MockProvider;

#[test]
fn test_reads_balance() {
    // block_on is still useful in tests — tests are sync by default.
    // (Or use #[tokio::test] — MockProvider works with any executor.)
    let mut mock = MockProvider::new(1);

    // Queue mock responses (FIFO)
    mock.push_success(&U256::from(1_000_000));   // for get_balance
    mock.push_success(&19_000_001u64);            // for get_block_number

    let provider = mock.provider();

    let balance = block_on(provider.get_balance(addr).latest()).unwrap();
    assert_eq!(balance, U256::from(1_000_000));

    let block = block_on(provider.get_block_number()).unwrap();
    assert_eq!(block, 19_000_001);
}
```

Note: `block_on` is still available and useful in test code where `#[test]` functions are synchronous. In module code, prefer `async fn on_event` with `.await` instead.

## Trade-Offs

| Concern | Generic passthrough | Per-method WIT functions |
|---|---|---|
| **WIT changes for new methods** | None | New function + types per method |
| **Host implementation** | ~20 lines total | Per-method impl + dispatch |
| **Guest API** | Full alloy Provider (80+ methods) | Only what WIT exposes |
| **alloy compatibility** | Native — IS an alloy transport | Manual ABI encode/decode |
| **Type safety at WIT boundary** | Runtime (JSON strings) | Compile-time (WIT types) |
| **Method allowlisting** | Runtime string match | Implicit (only exposed methods exist) |
| **Debugging** | JSON in/out visible in traces | Structured WIT types in traces |
| **Multi-language guests** | Must handle JSON serialisation | WIT types auto-generated |

The primary trade-off is **type safety at the WIT boundary**: JSON strings vs. structured WIT types. This is mitigated by:

1. **Rust guests** use alloy's type system — serialisation errors surface as alloy `TransportError` with clear messages.
2. **Non-Rust guests** (JS, Python, Go) typically work with JSON natively, so JSON strings are actually *more* natural than WIT record types.
3. **Tracing**: the host can log method + params as structured JSON before forwarding, providing equal or better debuggability.

The compile-time guarantee that a module can only call methods in the WIT is traded for a runtime allowlist. Given that the Component Model already provides structural sandboxing (the module can only call `csn::request`, not arbitrary network I/O), and the allowlist is enforced at the host boundary before any RPC call is made, this is a sound trade-off.

## Migration Path

If the current `blockchain` interface has already been implemented:

1. Add `csn` interface alongside `blockchain` (both in WIT world).
2. SDK defaults to `csn`-backed `provider()`. Raw `blockchain::*` functions still work.
3. Deprecation cycle: mark `blockchain` functions as deprecated in SDK docs.
4. Remove `blockchain` interface in the next WIT minor version bump.

If starting from scratch (recommended): implement `csn` only. Skip `blockchain` entirely.

## Summary

| Component | What Changes |
|---|---|
| **WIT** | Replace `blockchain` with `csn` (1 function). Add `identity` interface (accounts, sign, sign-typed-data). `headless-module` imports 6 interfaces: csn, identity, local-store, remote-store, msg, logging. Downstream distributions may add further domain interfaces in their own WIT packages. |
| **Host** | `CsnHost<I: Identity>` — one `csn::request` impl that forwards read-only methods to `provider.raw_request_dyn` and delegates signing methods (`eth_sendTransaction`, `eth_accounts`, `eth_signTypedData_v4`, `personal_sign`) to the `Identity` backend. One `identity::Host` impl delegating to the same backend. |
| **SDK** | `nexum-sdk`: `HostTransport` (alloy `Transport` impl), `provider()` constructor, `block_on()`, `IdentityClient` (typed identity wrapper). |
| **`#[nexum::module]` macro** | Named event handlers (`on_block`, `on_logs`, `on_timer`) with generated match dispatch. `async fn` support. Optional `&RootProvider` injection. |
| **Module author experience** | Full alloy `Provider` API via injected provider. Signing via `IdentityClient` or transparently through `csn::request` signing methods. No match boilerplate. No `block_on`. No manual ABI wrangling for RPC calls. |
| **Existing ABI helpers** | Unchanged — `sol!` macro and `alloy-sol-types` still used for contract calldata encoding/decoding. |
