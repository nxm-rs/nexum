# WebTransport Hierarchical Multiplexing Protocol

**Status:** Draft
**Version:** 0.1.0
**Author:** Nexum Core Team
**Related Issues:** #89 (System Architecture)

## Abstract

This specification defines a hierarchical multiplexing protocol for WebTransport-based communication between the Nexum browser extension and standalone application. The protocol uses three levels of multiplexing: session, tab streams, and request streams, eliminating the need for request ID management while providing natural state isolation and clean lifecycle management.

## 1. Introduction

### 1.1 Motivation

Traditional WebSocket-based RPC protocols require explicit request/response correlation using unique identifiers. This adds complexity to message handling, state management, and error recovery. WebTransport's native stream multiplexing capabilities can eliminate this overhead while providing better concurrency and state isolation.

### 1.2 Design Goals

1. **Zero ID Management:** Leverage streams as correlation mechanism
2. **Tab State Isolation:** Each browser tab maintains independent context
3. **Clean Lifecycle:** Automatic cleanup when tabs close
4. **Natural Concurrency:** Multiple concurrent requests per tab without blocking
5. **Security Context:** Proper isolation of permissions and identity per tab

### 1.3 Non-Goals

- Backward compatibility with WebSocket-based protocol
- Mobile browser support (initial implementation)
- P2P communication between browser tabs

## 2. Architecture Overview

### 2.1 Three-Level Hierarchy

```
┌─────────────────────────────────────────────────────────────┐
│ Level 1: WebTransport Session                              │
│   - Persistent connection: Extension ↔ Application         │
│   - Handles connection lifecycle, reconnection              │
│   - Certificate verification, security handshake            │
└─────────────┬───────────────────────────────────────────────┘
              │
         ┌────┴────┬────────┬────────┐
         ▼         ▼        ▼        ▼
┌──────────────────────────────────────────────────────────────┐
│ Level 2: Tab Streams (Persistent Bidirectional)             │
│   - One per browser tab                                      │
│   - Carries tab state: identity, chain, origin              │
│   - Remains open for lifetime of tab                         │
└─────────────┬────────────────────────────────────────────────┘
              │
         ┌────┴────┬────────┬────────┐
         ▼         ▼        ▼        ▼
┌──────────────────────────────────────────────────────────────┐
│ Level 3: Request Streams (Ephemeral Bidirectional)          │
│   - One per RPC call                                         │
│   - Short-lived: open, send request, receive response, close │
│   - Inherits context from parent tab stream                  │
└──────────────────────────────────────────────────────────────┘
```

### 2.2 Stream Types

| Stream Type | Direction | Lifetime | Purpose |
|------------|-----------|----------|---------|
| Session | Bidirectional | Connection duration | Transport layer |
| Tab Stream | Bidirectional | Tab lifetime | State container |
| Request Stream | Bidirectional | Single RPC call | Request/response |

## 3. Session Layer (Level 1)

### 3.1 Connection Establishment

```rust
// Extension side
async fn connect_to_application() -> Result<WebTransportSession> {
    let url = "https://127.0.0.1:1250";
    let options = WebTransportOptions {
        server_certificate_hashes: vec![expected_cert_hash()],
        ..Default::default()
    };

    WebTransport::connect(url, options).await
}
```

### 3.2 Certificate Pinning

The application serves a self-signed certificate. The extension MUST verify:
- Certificate hash matches expected value
- Certificate is valid (not expired)
- Connection is to localhost (127.0.0.1)

**Security Considerations:**
- Certificate hash MUST be embedded in extension at build time
- No user override for certificate validation
- Refuse connection on mismatch

### 3.3 Reconnection Strategy

On disconnection:
1. Close all tab streams and pending request streams
2. Wait exponential backoff: 1s, 2s, 4s, 8s, max 30s
3. Attempt reconnection
4. On success, recreate tab streams for active tabs
5. Notify user if reconnection fails after 5 attempts

### 3.4 Keep-Alive

- Send keep-alive datagrams every 30 seconds
- If no datagram received for 90 seconds, consider connection dead
- Initiate reconnection

## 4. Tab Stream Layer (Level 2)

### 4.1 Tab Stream Lifecycle

```
┌──────────────┐
│  Tab Opened  │
└──────┬───────┘
       │
       ▼
┌────────────────────┐
│ Create Tab Stream  │
│ Open bidirectional │
│ stream on session  │
└──────┬─────────────┘
       │
       ▼
┌─────────────────────────┐
│ Send InitializeTab      │
│ { tab_id, origin }      │
└──────┬──────────────────┘
       │
       ▼
┌─────────────────────────┐
│ Receive Acknowledgment  │
│ { status: "ready" }     │
└──────┬──────────────────┘
       │
       ▼
┌──────────────────────────┐
│ Stream Ready for         │
│ - State updates          │
│ - Creating request       │
│   streams                │
└──────┬───────────────────┘
       │
       │ (Tab active)
       │
       ▼
┌──────────────┐
│  Tab Closed  │
└──────┬───────┘
       │
       ▼
┌────────────────────┐
│ Send CloseTab      │
│ Close tab stream   │
└────────────────────┘
```

### 4.2 Tab Stream Message Format

All tab stream messages use CBOR encoding for efficiency.

#### 4.2.1 InitializeTab

Sent by extension when tab stream is first created.

```rust
struct InitializeTab {
    /// Unique identifier for this tab
    tab_id: TabId,

    /// Origin of the dapp (e.g., "https://uniswap.org")
    origin: String,

    /// Requested initial identity (optional)
    identity: Option<Identity>,

    /// Requested initial chain (optional)
    chain: Option<ChainId>,
}
```

**Application Response:**
```rust
struct TabInitialized {
    status: "ready",

    /// Actual identity assigned (may differ from requested)
    identity: Identity,

    /// Actual chain assigned
    chain: ChainId,
}
```

#### 4.2.2 SetIdentity

Sent by extension when user switches identity for this tab.

```rust
struct SetIdentity {
    identity: Identity,
}
```

**Application Response:**
```rust
struct IdentityChanged {
    identity: Identity,
    /// Whether dapp needs to reconnect
    requires_reconnect: bool,
}
```

#### 4.2.3 SetChain

Sent by extension when user switches chain for this tab.

```rust
struct SetChain {
    chain: ChainId,
}
```

**Application Response:**
```rust
struct ChainChanged {
    chain: ChainId,
}
```

#### 4.2.4 KeepAlive

Sent periodically (every 30s) to maintain stream.

```rust
struct KeepAlive {
    timestamp: u64,
}
```

**Application Response:**
```rust
struct KeepAliveAck {
    timestamp: u64,
}
```

#### 4.2.5 CloseTab

Sent when tab is closed.

```rust
struct CloseTab {
    reason: String,  // "user_closed", "navigation", "crash"
}
```

No response required. Stream is closed immediately after sending.

### 4.3 Tab Context State

The application maintains the following state per tab stream:

```rust
struct TabContext {
    /// Stream identifier
    tab_id: TabId,

    /// Current active identity
    identity: Identity,

    /// Current active chain
    chain: ChainId,

    /// Origin of the dapp
    origin: String,

    /// Permissions granted to this origin
    permissions: Permissions,

    /// Last activity timestamp
    last_activity: Instant,

    /// Pending request streams
    active_requests: HashMap<StreamId, RequestContext>,
}
```

## 5. Request Stream Layer (Level 3)

### 5.1 Request Stream Lifecycle

```
┌─────────────────────┐
│ RPC Call Initiated  │
│ (from dapp)         │
└──────┬──────────────┘
       │
       ▼
┌──────────────────────────┐
│ Open Request Stream      │
│ (bidirectional)          │
│ within tab's session     │
└──────┬───────────────────┘
       │
       ▼
┌──────────────────────────┐
│ Send Request Message     │
│ { tab_id, method,        │
│   params }               │
└──────┬───────────────────┘
       │
       ▼
┌──────────────────────────┐
│ Application processes    │
│ through security layers  │
│ - Firewall               │
│ - Risk assessment        │
│ - Interpretation         │
│ - Signing rules          │
│ - Execution validation   │
└──────┬───────────────────┘
       │
       ▼
┌──────────────────────────┐
│ Receive Response         │
│ { result } or { error }  │
└──────┬───────────────────┘
       │
       ▼
┌──────────────────────────┐
│ Close Request Stream     │
└──────────────────────────┘
```

### 5.2 Request Stream Message Format

#### 5.2.1 Request

```rust
struct Request {
    /// Tab identifier (links to parent tab stream)
    tab_id: TabId,

    /// JSON-RPC method name
    method: String,

    /// Method parameters
    params: serde_json::Value,
}
```

#### 5.2.2 Response (Success)

```rust
struct Response {
    /// Result data
    result: serde_json::Value,
}
```

#### 5.2.3 Response (Error)

```rust
struct ErrorResponse {
    error: ErrorInfo,
}

struct ErrorInfo {
    /// Error code (JSON-RPC standard)
    code: i32,

    /// Human-readable message
    message: String,

    /// Additional error data
    data: Option<serde_json::Value>,
}
```

### 5.3 Request Stream Timeout

- Default timeout: 30 seconds
- Extension MUST close stream if no response received within timeout
- Application SHOULD cancel processing if stream is reset by extension

## 6. Security Integration

### 6.1 Context Propagation

When a request stream is opened:

1. Extract `tab_id` from request message
2. Lookup `TabContext` from active tab streams
3. If tab context not found, reject with error:
   ```
   { code: -32000, message: "Tab context not found" }
   ```
4. Verify request stream belongs to correct session
5. Pass full context to security pipeline:
   ```rust
   struct SecurityContext {
       tab: TabContext,
       request: Request,
   }
   ```

### 6.2 Security Layer Flow

```
Request Stream Received
  │
  ├─▶ [Layer 1: Firewall]
  │   ├─ Check origin permissions
  │   ├─ Check identity permissions
  │   ├─ Check method allowlist
  │   ├─ Rate limit check
  │   └─▶ PASS or DENY
  │
  ├─▶ [Layer 2: Risk Assessment]
  │   ├─ Analyze transaction value
  │   ├─ Check destination address
  │   ├─ Evaluate calldata complexity
  │   └─▶ Calculate risk score
  │
  ├─▶ [Layer 3: Interpretation]
  │   ├─ Parse transaction components
  │   ├─ ABI decode function
  │   └─▶ Generate human-readable text
  │
  ├─▶ [Layer 4: Signing Rules]
  │   ├─ Evaluate auto-approval conditions
  │   ├─ Check identity-specific policies
  │   └─▶ AUTO-APPROVE or REQUIRE-CONFIRMATION
  │       │
  │       └─▶ [If REQUIRE-CONFIRMATION]
  │           ├─ Display TUI prompt
  │           ├─ Show human-readable description
  │           ├─ Show risk score
  │           └─ Wait for user decision
  │
  └─▶ [Layer 5: Execution Validation]
      ├─ Validate gas limits
      ├─ Check nonce
      ├─ Verify balance
      ├─ Simulate transaction
      └─▶ Execute or reject
```

### 6.3 Per-Tab Permissions

Each tab context maintains:

```rust
struct Permissions {
    /// Methods explicitly allowed
    allowed_methods: HashSet<String>,

    /// Methods explicitly denied
    denied_methods: HashSet<String>,

    /// Whether dapp can request identity switch
    can_request_identity_change: bool,

    /// Whether dapp can request chain switch
    can_request_chain_change: bool,

    /// Maximum transaction value auto-approved
    auto_approve_threshold: Option<U256>,
}
```

Permissions are:
- Established during `InitializeTab`
- Can be updated via user interaction in TUI
- Scoped to `(origin, identity)` pair

## 7. Error Handling

### 7.1 Stream Reset

**Extension Side:**
- If stream reset by application: treat as fatal error for that request
- Return error to dapp: `{ code: -32603, message: "Internal error" }`

**Application Side:**
- If request stream reset by extension: cancel pending operation
- Clean up resources (stop signing, release locks)
- No response needed (stream already closed)

### 7.2 Tab Stream Disconnect

If tab stream closes unexpectedly:

**Extension:**
- Attempt to recreate tab stream once
- If failed, notify dapp that connection is lost
- Queue pending requests until reconnected

**Application:**
- Cancel all pending requests for this tab
- Clean up tab context
- Wait for new `InitializeTab` if tab reconnects

### 7.3 Session Disconnect

If WebTransport session closes:

**Extension:**
- Close all tab streams
- Initiate reconnection (see §3.3)
- Notify all dapps of disconnection
- Queue all pending requests

**Application:**
- Close all tab streams
- Cancel all pending operations
- Clean up all resources
- Wait for new connection

## 8. Performance Considerations

### 8.1 Concurrent Requests

Each tab can have multiple concurrent request streams:
- Up to 100 concurrent request streams per tab
- If limit reached, queue additional requests
- Extension SHOULD implement request prioritization

### 8.2 Stream Creation Overhead

Creating a new bidirectional stream is lightweight (~100 bytes overhead).
For high-frequency calls:
- Consider request batching
- Keep statistics on method frequency
- Optimize hot paths

### 8.3 Buffer Management

- Tab stream: 64KB send buffer, 64KB receive buffer
- Request stream: 16KB send buffer, 16KB receive buffer
- Apply backpressure if buffers fill

## 9. Migration from WebSocket

### 9.1 Feature Detection

Extension detects application capabilities:
```javascript
try {
  // Try WebTransport first
  const session = await WebTransport.connect("https://127.0.0.1:1250");
  return new WebTransportProtocol(session);
} catch (e) {
  // Fall back to WebSocket
  const ws = await WebSocket.connect("ws://127.0.0.1:1250");
  return new WebSocketProtocol(ws);
}
```

### 9.2 Dual-Protocol Support

Application can support both simultaneously:
- WebTransport on port 1250 (HTTPS)
- WebSocket on port 1250 (HTTP upgrade)

## 10. Test Vectors

### 10.1 Tab Initialization

**Extension sends:**
```json
{
  "tab_id": "550e8400-e29b-41d4-a716-446655440000",
  "origin": "https://uniswap.org",
  "identity": null,
  "chain": null
}
```

**Application responds:**
```json
{
  "status": "ready",
  "identity": "alice.eth",
  "chain": 1
}
```

### 10.2 Simple Request

**Extension sends (on request stream):**
```json
{
  "tab_id": "550e8400-e29b-41d4-a716-446655440000",
  "method": "eth_blockNumber",
  "params": []
}
```

**Application responds:**
```json
{
  "result": "0x10d4f"
}
```

### 10.3 Transaction Request

**Extension sends:**
```json
{
  "tab_id": "550e8400-e29b-41d4-a716-446655440000",
  "method": "eth_sendTransaction",
  "params": [{
    "from": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb",
    "to": "0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed",
    "value": "0xde0b6b3a7640000",
    "gas": "0x5208",
    "gasPrice": "0x9184e72a000"
  }]
}
```

**Application may respond with:**
```json
{
  "error": {
    "code": -32000,
    "message": "User denied transaction"
  }
}
```

## 11. Security Considerations

### 11.1 Certificate Pinning

MUST pin application certificate to prevent MITM attacks on localhost.

### 11.2 Origin Isolation

Each tab stream is tied to exactly one origin. Cross-origin requests MUST be rejected.

### 11.3 Identity Isolation

Dapps cannot:
- Enumerate identities
- Access other tabs' contexts
- Determine which identity other tabs are using

### 11.4 Resource Limits

To prevent DoS:
- Maximum 50 tab streams per session
- Maximum 100 request streams per tab
- Maximum 1MB message size
- Rate limiting: 100 requests/second per tab

## 12. Future Extensions

### 12.1 Server-Initiated Events

Tab stream can be used for application → extension events:
- New block
- Network switch
- Identity added/removed

### 12.2 Subscription Streams

Long-lived request streams for event subscriptions:
- Keep stream open
- Send events as they arrive
- Close when unsubscribe

### 12.3 Batch Requests

Single request stream, multiple requests:
```json
{
  "batch": [
    { "method": "eth_getBalance", "params": ["0x..."] },
    { "method": "eth_getTransactionCount", "params": ["0x..."] }
  ]
}
```

## Appendix A: Type Definitions

```rust
/// Unique identifier for a browser tab
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct TabId(Uuid);

/// User identity (ENS name or address)
#[derive(Debug, Clone, PartialEq, Eq)]
struct Identity(String);

/// EVM chain ID
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ChainId(u64);
```

## Appendix B: References

- WebTransport Specification: https://w3c.github.io/webtransport/
- QUIC Protocol: RFC 9000
- JSON-RPC 2.0 Specification: https://www.jsonrpc.org/specification
- EIP-1193: Ethereum Provider JavaScript API

---

**Document History**

- 2025-11-08: Initial draft (v0.1.0)
