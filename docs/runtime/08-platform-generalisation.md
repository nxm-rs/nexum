# Platform Generalisation

## Motivation

The nexum runtime (docs 01-07) is designed as a server-side Rust binary embedding wasmtime. But the core abstractions — WIT-defined host interfaces, content-addressed module distribution, declarative manifests — are not inherently server-specific. The same module binary, the same packaging, and the same distribution mechanism can serve multiple platform targets:

1. **Server runtime** — the current design (Rust/Tokio/wasmtime). Headless automation: blockchain event monitoring, order submission, background computation.
2. **Mobile app (Flutter/Dart)** — a WASM runtime embedded in a native mobile application via FFI. Modules run on-device, backed by local state (SQLite) and RPC over HTTP.
3. **WebView** — a browser engine (V8/JSC/SpiderMonkey) executing WASM natively, with host functions injected from the native layer via a JavaScript bridge. Enables rich web-based UIs with blockchain-native capabilities.
4. **Decentralised super app** — a shell application (mobile or desktop) that dynamically loads modules discovered via ENS and fetched from Swarm. Some modules are headless (automation); others are interactive (UI). All are sandboxed, all are distributed without a central app store.

The key insight: **the WIT contract is the universal interface**. Any host that implements the required interfaces can run the same module binary. The differences between platforms are in *how* the host implements those interfaces — not in what the module sees.

This document defines the layered architecture that enables this generalisation and specifies the universal interface set.

## Primitive Taxonomy

Before diving into WIT definitions, the universal runtime is built on six primitive capabilities. These are the fundamental building blocks that any decentralised application needs:

| Primitive | Interface | Backed by | Purpose |
|-----------|-----------|-----------|---------|
| **Consensus** | `csn` | JSON-RPC (eth_*) | Read/write blockchain consensus state |
| **Identity** | `identity` | Keystore / KMS / device keychain / wallet extension | Cryptographic identity — key management and signing |
| **Local Store** | `local-store` | redb / SQLite / IndexedDB | Per-module private persistence on the device |
| **Remote Store** | `remote-store` | Ethereum Swarm | Decentralised content-addressed storage |
| **Messaging** | `msg` | Waku | Decentralised pub/sub messaging |
| **Logging** | `logging` | tracing / console | Diagnostic output |

These six primitives are orthogonal:

- **Consensus** is the source of truth — the blockchain. Modules read chain state and (indirectly) write to it via order submission or transactions.
- **Identity** is cryptographic agency — key management and signing. Modules can enumerate available accounts and request signatures (ECDSA secp256k1 by default, extensible). The `csn` host implementation depends on `identity` internally — signing RPC methods (e.g. `eth_sendTransaction`) delegate to `identity` for the actual signature.
- **Local Store** is the module's private scratchpad — fast, local, scoped to one module on one device. Does not replicate.
- **Remote Store** is shared persistent content — content-addressed, decentralised, survives independent of any device. Any module on any device can read what another module wrote.
- **Messaging** is real-time communication — ephemeral pub/sub messages between modules, devices, or users. Unlike remote store (persistent, content-addressed), messaging is transient and topic-based.
- **Logging** is diagnostics — one-way output for debugging and monitoring. Not a data channel.

Together they cover the full spectrum: persistent truth (consensus), cryptographic agency (identity), local scratch (local-store), shared content (remote-store), real-time coordination (msg), and diagnostics (logging).

## Architectural Principle: Layered WIT Worlds

To enable reuse across platforms and domains, the WIT is split into layers. nexum itself ships only Layer 1 (the universal runtime interfaces). Layer 2 and Layer 3 are **optional** layers that downstream distributions or application authors may define on top:

```mermaid
graph TD
    subgraph L3["Layer 3: Application-Specific Worlds (downstream)"]
        COW["shepherd:cow — cow + order (CoW Protocol automation)"]
        DEFI["myapp:defi — vault + strategy (DeFi yield app)"]
        GAME["game:engine — physics + assets (on-chain game)"]
    end

    subgraph L2["Layer 2: Capability Extensions (optional, composable)"]
        UI["ui — user interface bridge (interactive modules)"]
    end

    subgraph L1["Layer 1: Universal Runtime Interfaces (nexum)"]
        CSN["csn — consensus access (JSON-RPC passthrough)"]
        ID["identity — cryptographic identity (key management, signing)"]
        LS["local-store — local key-value persistence"]
        RS["remote-store — decentralised content-addressed storage"]
        MSG["msg — decentralised pub/sub messaging"]
        LOG["logging — structured logging"]
        EXP["Exports: init(config) + on-event(event)"]
    end

    L3 -->|"builds on via WIT include"| L2
    L2 -->|"builds on via WIT include"| L1
```

Each layer builds on the one below via WIT `include`. A module compiled against Layer 1 alone runs on any conforming host. A module compiled against a Layer 3 package (e.g. `shepherd:cow`, which is the downstream Shepherd distribution's CoW Protocol extension) requires a host that implements Layers 1 + that extension.

## Layer 1: Universal Interfaces

These six interfaces form the universal runtime contract. Any platform — server, mobile, WebView, desktop — can implement them.

### `csn` — Consensus Access

The module's window into blockchain consensus. A single generic function that forwards JSON-RPC requests to the host's provider infrastructure. The host decides *how* to reach the chain — the module only specifies *what* to ask.

```wit
interface csn {
    type chain-id = u64;

    record json-rpc-error {
        code: s64,
        message: string,
        data: option<string>,
    }

    /// Execute a JSON-RPC request against the specified chain.
    ///
    /// The host routes to its configured provider for the given chain,
    /// applying whatever middleware is appropriate for the platform
    /// (timeout, retry, rate-limit, fallback on server; simple HTTP
    /// on mobile; window.ethereum or injected provider in WebView).
    ///
    /// `method` includes the namespace prefix (e.g. "eth_call").
    /// `params` and the success value are JSON-encoded strings.
    request: func(chain-id: chain-id, method: string, params: string)
        -> result<string, json-rpc-error>;
}
```

**Platform implementations:**

| Platform | `csn::request` backed by |
|----------|--------------------------|
| Server (nexum) | alloy provider with tower middleware (timeout, retry, rate-limit, fallback) |
| Mobile (Flutter) | HTTP client (reqwest via FFI, or Dart `http` package) to configured RPC endpoint |
| WebView | JavaScript bridge -> `window.ethereum` (injected wallet) or native HTTP via message channel |
| Super app | Same as mobile, with per-module chain permissions |

The Rust SDK's `HostTransport` (doc 07) works identically on all platforms — it implements alloy's `Transport` trait over `csn::request`, so module authors get the full alloy `Provider` API regardless of where the module runs.

### `identity` — Cryptographic Identity

Provides key management and signing capabilities to modules. ECDSA secp256k1 by default (the Ethereum standard), extensible to other schemes. Modules can enumerate available accounts and request signatures over arbitrary data.

The `csn` host implementation depends on `identity` internally — signing RPC methods such as `eth_sendTransaction` or `eth_signTypedData_v4` delegate to `identity` for the actual cryptographic signature. Modules can also import `identity` directly for raw signing operations outside of JSON-RPC (e.g. signing EIP-712 typed data for off-chain order submission).

```wit
interface identity {
    record identity-error {
        code: u16,
        message: string,
    }

    /// List available accounts (public keys or addresses).
    /// Returns a list of account identifiers (e.g. 20-byte Ethereum addresses).
    accounts: func() -> result<list<list<u8>>, identity-error>;

    /// Sign arbitrary data with the specified account's private key.
    /// Returns the signature bytes (e.g. 65-byte ECDSA signature with recovery id).
    sign: func(account: list<u8>, data: list<u8>) -> result<list<u8>, identity-error>;

    /// Sign EIP-712 typed structured data.
    /// `typed-data` is the JSON-encoded EIP-712 typed data structure.
    /// Returns the signature bytes.
    sign-typed-data: func(account: list<u8>, typed-data: string) -> result<list<u8>, identity-error>;
}
```

**Platform implementations:**

| Platform | `identity` backed by |
|----------|---------------------|
| Server (nexum) | Keystore file, AWS KMS, or HSM |
| Mobile (Flutter) | Device keychain (Keystore/Keychain) or wallet SDK |
| WebView | window.ethereum (wallet extension) or native bridge to keychain |
| Super app | Device keychain + per-module permission grants |

**Relationship with `csn`:**

The `csn` host implementation uses `identity` internally when it encounters signing methods. For example, when a module calls `csn::request` with `eth_sendTransaction`, the host:

1. Constructs the transaction from the JSON-RPC params.
2. Calls `identity::sign` to produce the signature.
3. Sends the signed transaction via the provider.

This means modules that only need to sign transactions via standard JSON-RPC methods do not need to import `identity` directly — `csn` handles it transparently. Modules that need raw signing (e.g. off-chain message signing for order submission, attestations, or custom protocols) import `identity` explicitly.

### `local-store` — Local Key-Value Persistence

The module's private scratchpad. **Local to the device/process** — does not replicate, sync, or share across instances. Scoped to one module: module A cannot read module B's local state.

```wit
interface local-store {
    /// Get a value by key. Returns None if the key does not exist.
    get: func(key: string) -> result<option<list<u8>>, string>;

    /// Set a key-value pair. Overwrites any existing value.
    /// The host MAY enforce a size quota; if exceeded, returns Err.
    set: func(key: string, value: list<u8>) -> result<_, string>;

    /// Delete a key. No-op if the key does not exist.
    delete: func(key: string) -> result<_, string>;

    /// List all keys matching a prefix. Empty prefix returns all keys.
    list-keys: func(prefix: string) -> result<list<string>, string>;
}
```

**Platform implementations:**

| Platform | `local-store` backed by |
|----------|-------------------------|
| Server (nexum) | redb (per-module database file, ACID, MVCC) |
| Mobile (Flutter) | SQLite (per-module table or database, via `sqflite`) |
| WebView | IndexedDB (per-module object store) or `localStorage` |
| Super app | SQLite (shared database, per-module namespace isolation) |

The semantics are deliberately minimal — get, set, delete, prefix scan. This is the LCD (lowest common denominator) that every platform can implement efficiently. Advanced features (transactions, MVCC, crash-safety) are host-specific and not exposed in the WIT.

The server runtime's all-or-nothing transactional semantics (doc 04) remain an implementation detail of the nexum host, not a guarantee modules can rely on across platforms. Modules that need stronger guarantees should design for idempotency.

### `remote-store` — Decentralised Content-Addressed Storage

Backed by Ethereum Swarm. Provides decentralised persistence beyond the local device — content-addressed, censorship-resistant, and accessible from any host on any device.

Swarm is both the distribution mechanism (modules are fetched from Swarm) and a runtime capability. This interface closes the loop — modules can publish to the same network they were distributed through.

```wit
interface remote-store {
    record store-error {
        code: u16,
        message: string,
    }

    /// Upload raw data to the decentralised store.
    /// Returns the 32-byte content reference (Swarm address).
    ///
    /// The host routes to its configured Bee node. Postage batch
    /// management is the host's responsibility — the module only
    /// provides data and gets back a reference.
    upload: func(data: list<u8>) -> result<list<u8>, store-error>;

    /// Download raw data by 32-byte content reference.
    ///
    /// The host fetches from its Bee node or a public gateway.
    /// Returns the raw bytes. The caller is responsible for
    /// interpreting the content (JSON, protobuf, WASM, etc.).
    download: func(reference: list<u8>) -> result<list<u8>, store-error>;

    /// Read the latest value from a mutable feed.
    ///
    /// Feeds are mutable pointers: (owner, topic) -> latest chunk.
    /// `owner`: 20-byte Ethereum address of the feed owner.
    /// `topic`: 32-byte topic hash.
    ///
    /// Returns None if the feed has no updates.
    feed-get: func(
        owner: list<u8>,
        topic: list<u8>,
    ) -> result<option<list<u8>>, store-error>;

    /// Update a mutable feed with new data.
    ///
    /// The host signs the feed update with its configured identity
    /// (Bee node's Ethereum key). Only the host's own feeds can be
    /// updated — the owner is implicit (the host's address).
    ///
    /// `topic`: 32-byte topic hash.
    /// `data`: the payload to publish.
    ///
    /// Returns the 32-byte reference of the new chunk.
    feed-set: func(
        topic: list<u8>,
        data: list<u8>,
    ) -> result<list<u8>, store-error>;
}
```

**Platform implementations:**

| Platform | `remote-store` backed by |
|----------|--------------------------|
| Server (nexum) | Direct Bee API (`http://localhost:1633`) |
| Mobile (Flutter) | Bee API via HTTP (local light node or remote gateway) |
| WebView | JavaScript bridge -> native HTTP to Bee gateway |
| Super app | Embedded Bee light node or gateway proxy |

**Why remote-store as a universal interface:**

- **Decentralised persistence.** `local-store` is device-local. `remote-store` gives modules access to content-addressed storage that persists independent of any single device.
- **Content distribution.** Modules can publish data (feeds, references) that other modules or users can consume — without a central server.
- **Cross-device coordination.** Two instances of the same module on different devices can share data via feed topics — one writes via `feed-set`, the other reads via `feed-get`.
- **Consistency with distribution model.** Modules are already fetched from Swarm (doc 02, 03). Exposing `remote-store` at runtime means modules participate in the same content-addressed network they were distributed through.

### `msg` — Decentralised Messaging

Backed by Waku. Provides real-time, privacy-preserving pub/sub messaging between modules, devices, and users. Unlike `remote-store` (persistent, content-addressed), `msg` is transient and topic-based — fire-and-forget messages on content topics.

```wit
interface msg {
    record msg-error {
        code: u16,
        message: string,
    }

    record message {
        content-topic: string,
        payload: list<u8>,
        timestamp: u64,
        /// Optional sender identity (protocol-dependent).
        sender: option<list<u8>>,
    }

    /// Publish a message to a content topic.
    ///
    /// The host routes to its configured Waku node. The message is
    /// propagated to all subscribers of the content topic via the
    /// Waku relay (gossipsub) or light push protocol.
    ///
    /// Content topics follow the format: /<app>/<version>/<topic>/<encoding>
    /// e.g. "/nexum/1/block-updates/proto"
    publish: func(content-topic: string, payload: list<u8>) -> result<_, msg-error>;

    /// Query historical messages from the Waku store protocol.
    ///
    /// Returns messages matching the content topic within the
    /// optional time range. Not all hosts support store queries
    /// (depends on Waku node configuration).
    query: func(
        content-topic: string,
        start-time: option<u64>,
        end-time: option<u64>,
        limit: option<u32>,
    ) -> result<list<message>, msg-error>;
}
```

**Receiving messages** is handled through the event system, not the `msg` interface. Modules declare message subscriptions in their manifest, and the host delivers them as events:

```toml
[[subscribe]]
type = "message"
content_topic = "/nexum/1/block-updates/proto"
```

The event variant is extended to include message events:

```wit
record message-data {
    content-topic: string,
    payload: list<u8>,
    timestamp: u64,
    sender: option<list<u8>>,
}

variant event {
    block(block-data),
    logs(list<log-entry>),
    timer(u64),
    message(message-data),
}
```

This follows the same pattern as all other event sources: sending uses the import interface (`msg::publish`), receiving uses the declarative subscription + `on-event` dispatch.

**Platform implementations:**

| Platform | `msg` backed by |
|----------|-----------------|
| Server (nexum) | Waku node (nwaku or go-waku) via JSON-RPC or REST API |
| Mobile (Flutter) | Waku light client via FFI (libwaku) or HTTP to remote Waku node |
| WebView | JavaScript bridge -> native Waku client, or js-waku in-browser |
| Super app | Embedded Waku light node |

**Why messaging as a universal interface:**

- **Module-to-module communication.** Two modules on different devices can exchange real-time messages via shared content topics. A headless automation module on a server can notify a mobile dashboard module that new data is available.
- **User notifications.** A headless server module can publish an alert to a content topic; the user's mobile app module subscribes and displays a notification.
- **Decentralised coordination.** Multiple instances of the same module (e.g. running on different operator nodes) can coordinate via messaging — leader election, work distribution, heartbeats.
- **Privacy.** Waku supports encrypted messaging and ephemeral relay. Modules can communicate without exposing data to the public chain.
- **Complementary to remote-store.** `remote-store` is for persistent content (data that should survive). `msg` is for ephemeral signals (notifications, coordination, real-time feeds). Together they cover the full persistence spectrum.

### `logging` — Structured Logging

Unchanged from the current design:

```wit
interface logging {
    enum level { trace, debug, info, warn, error }

    /// Emit a structured log message.
    /// The host decides how to handle it (stdout, file, discard).
    log: func(level: level, message: string);
}
```

Every platform implements this trivially. On server: `tracing` crate. On mobile: platform logger (`android.util.Log`, `os_log`). In WebView: `console.log`. The SDK's `info!`, `debug!`, etc. macros compile to this.

### Universal World Definition

```wit
package web3:runtime@0.1.0;

interface types {
    type chain-id = u64;

    record block-data {
        chain-id: chain-id,
        number: u64,
        hash: list<u8>,
        timestamp: u64,
    }

    record log-entry {
        chain-id: chain-id,
        address: list<u8>,
        topics: list<list<u8>>,
        data: list<u8>,
        block-number: u64,
        tx-hash: list<u8>,
        log-index: u32,
    }

    record message-data {
        content-topic: string,
        payload: list<u8>,
        timestamp: u64,
        sender: option<list<u8>>,
    }

    variant event {
        block(block-data),
        logs(list<log-entry>),
        timer(u64),
        message(message-data),
    }

    type config = list<tuple<string, string>>;
}

// ... csn, identity, local-store, remote-store, msg, logging interfaces as above ...

/// Headless module — automation, background processing.
/// No UI capabilities. Runs on any conforming host.
world headless-module {
    import csn;
    import identity;
    import local-store;
    import remote-store;
    import msg;
    import logging;

    export init: func(config: types.config) -> result<_, string>;
    export on-event: func(event: types.event) -> result<_, string>;
}
```

A module compiled against `web3:runtime/headless-module` is the **maximally portable** artifact. It runs on server, mobile, and WebView hosts without modification.

## Layer 2: UI Interface

Interactive modules — those with a user-facing presence in a super app or WebView container — import the `ui` interface in addition to the Layer 1 universals.

### Design Approach

The `ui` interface is a **bridge**, not a rendering engine. It does not define a widget tree, layout system, or styling language. Instead, it provides the communication channel between the module's logic (running in WASM) and the host's UI surface (a WebView, native view, or terminal).

For WebView-based hosts (the primary target for interactive modules), the module's UI is a web application (HTML/CSS/JS) served into a WebView by the host. The `ui` interface gives the module control over this surface and access to native capabilities that a normal web page cannot reach.

```wit
interface ui {
    record ui-error {
        code: u16,
        message: string,
    }

    /// Emit a UI update.
    ///
    /// For WebView hosts: `content` is an HTML fragment or a JSON
    /// message that the WebView's JavaScript layer interprets.
    /// For native hosts: `content` is a declarative description
    /// (format negotiated via host-info).
    ///
    /// The `target` identifies which UI surface to update
    /// (e.g. "main", "overlay", "notification-badge").
    render: func(target: string, content: string) -> result<_, ui-error>;

    /// Request navigation to a different view or module.
    ///
    /// `target`: a route string (e.g. "/settings", "module:price-alert").
    /// `params`: key-value parameters for the target.
    navigate: func(
        target: string,
        params: list<tuple<string, string>>,
    ) -> result<_, ui-error>;

    /// Show a native notification (outside the WebView).
    notify: func(title: string, body: string) -> result<_, ui-error>;

    /// Prompt the user for a yes/no decision via native dialog.
    confirm: func(title: string, body: string) -> result<bool, ui-error>;

    /// Query the host's UI capabilities.
    record host-capabilities {
        /// "android" | "ios" | "desktop" | "web" | "terminal"
        platform: string,
        /// Content format the host expects for render().
        /// "html" | "json-widget" | "markdown"
        render-format: string,
        supports-notifications: bool,
        supports-biometric: bool,
    }

    host-info: func() -> host-capabilities;
}
```

### Module Exports for Interactive Modules

Interactive modules export additional lifecycle hooks beyond `init` and `on-event`:

```wit
/// Interactive module — has a UI presence.
world app-module {
    include headless-module;
    import ui;

    /// Called when the module's UI surface is first displayed.
    /// Returns initial content to render.
    export on-render: func() -> result<string, string>;

    /// Called when the user interacts with a UI element.
    ///
    /// `element-id`: identifier of the element (set by the module in its render output).
    /// `action`: interaction type ("click", "submit", "change", etc.).
    /// `data`: optional payload (form data, input value, etc.).
    export on-interact: func(
        element-id: string,
        action: string,
        data: option<string>,
    ) -> result<_, string>;
}
```

This creates a bidirectional loop:

```mermaid
flowchart TD
    A["Host calls on-render"] --> B["Module returns initial UI content"]
    B --> C["Host displays in WebView/native surface"]
    C --> D["User interacts"]
    D --> E["Host calls on-interact(element, action, data)"]
    E --> F["Module processes interaction"]
    F --> G["module calls ui::render(target, new-content) to update UI"]
    F --> H["module calls csn::request to read chain state"]
    F --> I["module calls local-store::set to persist"]
    G --> C
```

The module's logic runs in the WASM sandbox. The UI runs in the WebView (or native surface). The `ui` interface is the bridge between them. This is analogous to Elm's update loop or React's message-passing model, but across the WASM-host boundary.

### WebView Module Packaging

A WebView-based interactive module bundles its web assets alongside the WASM component:

```
price-dashboard/
├── nexum.toml             # manifest (declares world: app-module)
├── module.wasm            # compiled WASM component
└── ui/
    ├── index.html         # entry point (loaded into WebView)
    ├── app.js             # UI logic (receives on-interact, calls render)
    └── style.css          # styling
```

The host loads `index.html` into a WebView and injects the bridge JavaScript that connects DOM events to `on-interact` and `ui::render` calls to DOM updates.

## Layer 3: Downstream Extensions

Domain-specific interfaces extend the universal layer for particular use cases. These live in **downstream** WIT packages — nexum itself does not ship any Layer 3 extensions. The pattern:

```wit
// Example: the downstream shepherd:cow package (part of the Shepherd
// distribution, not part of nexum itself). Shown here only to illustrate
// how third-party distributions extend the universal world.
package shepherd:cow@0.1.0;

interface cow {
    use web3:runtime/types.{chain-id};

    record api-error {
        status: u16,
        message: string,
        body: option<string>,
    }

    request: func(
        chain-id: chain-id,
        method: string,
        path: string,
        body: option<string>,
    ) -> result<string, api-error>;
}

interface order {
    use web3:runtime/types.{chain-id};

    submit: func(chain-id: chain-id, order-data: list<u8>)
        -> result<string, string>;
}

world shepherd-module {
    include web3:runtime/headless-module;
    import cow;
    import order;
}
```

Other domains follow the same pattern:

```wit
// Hypothetical DeFi yield module
package defi:yield@0.1.0;

interface vault { /* ... */ }
interface strategy { /* ... */ }

world yield-module {
    include web3:runtime/headless-module;
    import vault;
    import strategy;
}
```

The `include` mechanism ensures that any domain-specific module inherits the full universal interface set. A Layer 3 module world can call `csn::request`, `identity::sign`, `local-store::get`, `remote-store::upload`, `msg::publish`, and `logging::log` — plus whatever domain-specific interfaces its own world imports.

## Complete WIT Package Layout

nexum ships a single WIT package, `web3:runtime`:

```
wit/
└── web3-runtime/
    ├── types.wit              # chain-id, block-data, log-entry, message-data, event, config
    ├── csn.wit                # csn interface (consensus access)
    ├── identity.wit           # identity interface (key management, signing)
    ├── local-store.wit        # local-store interface
    ├── remote-store.wit       # remote-store interface (Swarm)
    ├── msg.wit                # msg interface (Waku)
    ├── logging.wit            # logging interface
    ├── ui.wit                 # ui interface + host-capabilities
    ├── headless-module.wit    # headless-module world
    └── app-module.wit         # app-module world (includes ui)
```

Downstream distributions keep their own WIT packages in their own repositories — for example, Shepherd maintains `shepherd:cow` in the Shepherd repository, not in nexum. New domains add new packages without touching the universal layer.

## Platform Targets

### Server Runtime (Reference Implementation — nexum)

This is the current design (docs 01-07), adapted for the layered WIT.

| Interface | Implementation |
|-----------|---------------|
| `csn` | alloy provider with tower middleware (timeout, retry, rate-limit, fallback) |
| `identity` | Keystore file, AWS KMS, or HSM — operator-configured signing backend |
| `local-store` | redb (per-module database file, ACID, MVCC, crash-safe) |
| `remote-store` | Bee API (`http://localhost:1633`) — operator runs a Bee node |
| `msg` | Waku node (nwaku) via JSON-RPC or REST API |
| `logging` | `tracing` crate -> JSON structured logs |
| Event sources | `eth_subscribe` (blocks, logs), cron (Tokio interval), Waku relay (messages) |
| WASM engine | wasmtime 41.x (Component Model, fuel, epoch metering) |

Downstream distributions add their own host implementations for their extension interfaces (for example, Shepherd adds a `cow` host that forwards to the CoW Protocol REST API).

### Mobile App (Flutter/Dart)

A Flutter application embeds a WASM runtime and provides the universal interfaces via Dart implementations:

```mermaid
flowchart TD
    subgraph FlutterApp["Flutter App"]
        subgraph WASMRuntime["WASM Runtime (via FFI)"]
            ENGINE["wasmtime C API or wasmer_dart or wasm3 (lightweight, C-based)"]
            ModA["Module A (headless)"]
            ModB["Module B (headless)"]
        end

        subgraph HostAdapter["Host Adapter (Dart)"]
            HA_CSN["csn -> HTTP client to RPC endpoint"]
            HA_ID["identity -> device keychain (Keystore/Keychain) or wallet SDK"]
            HA_LS["local-store -> SQLite (sqflite)"]
            HA_RS["remote-store -> HTTP to Bee gateway"]
            HA_MSG["msg -> libwaku via FFI"]
            HA_LOG["logging -> platform logger"]
        end

        subgraph EventSources["Event Sources (Dart)"]
            ES["Block polling (HTTP, no WebSocket on mobile background), timer via Dart Timer, Waku subscription via light client, push notifications (optional)"]
        end

        ModA --> HostAdapter
        ModB --> HostAdapter
        HostAdapter --> EventSources
    end
```

**WASM engine options:**

| Engine | Component Model | Mobile support | Notes |
|--------|----------------|----------------|-------|
| wasmtime (C API) | Full | aarch64 (iOS/Android ARM64) | Best compatibility, largest binary size (~15 MB) |
| wasmer | Partial | Good (wasmer_dart exists) | Component Model support is partial |
| wasm3 | None | Excellent (tiny C library, ~100 KB) | Interpreter only, no Component Model — requires core module + shim |

For full Component Model support (identical module binaries across server and mobile), **wasmtime via C API** is the recommended path. Dart's FFI (`dart:ffi`) can call the wasmtime C API directly. The binary size cost (~15 MB) is acceptable for a mobile app.

**Mobile-specific constraints:**

- **Background execution.** iOS and Android aggressively suspend background processes. A mobile host cannot maintain persistent WebSocket subscriptions. Event sourcing must be adapted: poll on foreground, use push notifications or local alarms for time-sensitive events.
- **Battery.** Continuous block polling drains battery. The mobile host should use adaptive polling intervals and batch event processing.
- **Connectivity.** Mobile networks are intermittent. Host functions should handle offline gracefully (queue requests, retry on reconnect).
- **Waku light client.** Mobile devices should use Waku's light push and filter protocols rather than full relay to minimise bandwidth and battery consumption.

### WebView (Browser Engine + Injected Host Functions)

A WebView host runs inside a native app (or standalone browser). The WASM module executes in the browser's native WASM engine. Host functions are injected via a JavaScript bridge.

```mermaid
flowchart TD
    subgraph NativeApp["Native App Shell"]
        subgraph WebView["WebView"]
            WASMModule["WASM Module (browser's WASM engine)\nCalls imported functions:\ncsn.request(...)\nidentity.sign(...)\nlocalStore.get(...)\nremoteStore.download(...)\nmsg.publish(...)\nlogging.log(...)"]

            subgraph JSBridge["JavaScript Bridge (injected)"]
                JS["window.web3runtime = {\n  csn: { request: (c, m, p) =>\n    nativeBridge.call('csn', ...) },\n  identity: { accounts: () =>\n    nativeBridge.call('identity', ...) },\n  localStore: { get: (k) =>\n    nativeBridge.call('store', ..) },\n  remoteStore: { download: (ref) =>\n    nativeBridge.call('store', ..) },\n  msg: { publish: (t, p) =>\n    nativeBridge.call('msg', ...) },\n  logging: { log: (l, m) =>\n    console.log(${`[l] m`}) }\n}"]
            end

            WASMModule --> JSBridge
        end

        subgraph NativeHost["Native Host Adapter"]
            NH_CSN["csn -> HTTP to RPC / wallet bridge"]
            NH_ID["identity -> window.ethereum / native keychain"]
            NH_LS["local-store -> SQLite / IndexedDB"]
            NH_RS["remote-store -> HTTP to Bee gateway"]
            NH_MSG["msg -> Waku node / js-waku"]
            NH_LOG["logging -> native logger"]
        end

        JSBridge -->|"message channel"| NativeHost
    end
```

**Component Model in the browser:**

Browsers don't natively support the WASM Component Model (as of early 2026). Two approaches:

1. **`jco` transpilation** (recommended). The Bytecode Alliance's `jco` tool transpiles a WASM component to a core WASM module + JavaScript glue code. The JS glue implements the canonical ABI marshalling. The result runs in any browser. This means the **same `.wasm` component** built for the server can be transpiled and run in a WebView.

2. **Core module variant.** Compile the module as a core WASM module (not a component) with a JS shim layer that maps the WIT interface to JavaScript imports. This requires a separate build target but avoids the `jco` dependency.

Approach 1 is preferred — it preserves the single-artifact property (one `.wasm` component, multiple platforms).

**WebView-specific capability: `window.ethereum`**

In a browser context, the user may have a wallet extension (MetaMask, Rabby, etc.) that injects `window.ethereum`. The `csn::request` host function can optionally route through this:

```javascript
// In the JS bridge
csn: {
    request: async (chainId, method, params) => {
        if (window.ethereum && useWalletProvider) {
            // Route through user's wallet (gets signing capabilities too)
            return await window.ethereum.request({ method, params: JSON.parse(params) });
        } else {
            // Route through native bridge to configured RPC endpoint
            return await nativeBridge.call('csn', { chainId, method, params });
        }
    }
}
```

This is powerful: the same module that runs headless on a server (reading chain state via a configured RPC endpoint) can run in a WebView and read chain state via the user's wallet — gaining access to the user's connected accounts and signing capabilities.

Similarly, the `identity` interface in a WebView context can delegate to `window.ethereum` for account enumeration and signing, providing a seamless bridge between the module's signing needs and the user's wallet extension.

**WebView-specific capability: `js-waku`**

For messaging in the browser, `js-waku` provides a pure JavaScript Waku client. The `msg` host function can route through `js-waku` directly in the WebView without needing the native bridge — peer-to-peer messaging from the browser.

### Decentralised Super App

The super app is the convergence of all targets. A native shell (Flutter) that:

1. **Discovers modules** via ENS (doc 03) — the same discovery mechanism as the server runtime.
2. **Fetches modules** from Swarm/IPFS — the same content-addressed distribution.
3. **Runs headless modules** in an embedded WASM runtime (automation, background tasks).
4. **Runs interactive modules** in WebViews (UI, dashboards, transaction builders).
5. **Provides the universal interfaces** to all modules (csn, identity, local-store, remote-store, msg, logging).
6. **Provides the UI interface** to interactive modules.

```mermaid
flowchart TD
    subgraph SuperApp["Super App Shell (Flutter)"]
        subgraph ModMgr["Module Manager"]
            DISC["Discovery: ENS -> Swarm -> content store -> verify"]
            LIFE["Lifecycle: Load -> Init -> Run -> Restart -> Dead"]
            PERM["Permissions: per-module capability grants"]
        end

        subgraph HeadlessRT["Headless WASM Runtime"]
            BL["Block Logger"]
            PRICE["Price Alert"]
        end

        subgraph WebViewPool["WebView Pool"]
            PORTFOLIO["Portfolio Dashboard (HTML)"]
            DEX["DEX Swap Interface (HTML)"]
        end

        subgraph HostLayer["Host Adapter Layer"]
            HL_CSN["csn -> HTTP to RPC endpoints"]
            HL_ID["identity -> device keychain + per-module grants"]
            HL_LS["local-store -> SQLite"]
            HL_RS["remote-store -> Bee light node / gateway"]
            HL_MSG["msg -> Waku light client"]
            HL_LOG["logging -> app logger + optional cloud"]
            HL_UI["ui -> WebView bridge (interactive modules)"]
        end

        subgraph ShellUI["Shell UI (Flutter)"]
            SHELL["Module gallery - Navigation - Settings - Wallet"]
        end

        ModMgr --> HeadlessRT
        ModMgr --> WebViewPool
        BL --> HostLayer
        PRICE --> HostLayer
        PORTFOLIO --> HostLayer
        DEX --> HostLayer
    end
```

**What makes this different from Telegram/WeChat mini-programs:**

| Aspect | Telegram/WeChat | Decentralised Super App |
|--------|-----------------|------------------------|
| Distribution | Central app store / bot platform | ENS -> Swarm/IPFS (no gatekeeper) |
| Integrity | Trust the platform | Content-addressed (hash-verified) |
| Execution | JavaScript in WebView (unrestricted) | WASM sandbox (capability-based) |
| Capabilities | Platform APIs (payments, camera, etc.) | Blockchain-native (consensus, identity, state, messaging) |
| Updates | Platform-mediated | Author updates ENS -> instant propagation |
| Censorship resistance | Platform can ban apps | ENS + Swarm = no single point of removal |
| Interoperability | Walled garden | Modules from any author, any domain |
| Communication | Platform's messaging API | Waku (decentralised, privacy-preserving) |

**Permissions model:**

The super app adds a capability-grant layer on top of the WIT world. When a module is installed, the user reviews what it imports:

```
"Price Alert" requests:
  ✓ csn          — read blockchain state (chains: 1, 42161)
  ✓ identity     — sign with your accounts
  ✓ local-store  — store data on your device
  ✓ remote-store — read/write to Swarm network
  ✓ msg          — send/receive messages (topics: /nexum/1/price-*)
  ✗ ui           — (not requested — headless module)

  [Allow]  [Deny]
```

The host only links interfaces the user has approved. A module that doesn't import `msg` structurally cannot publish messages — the same structural sandboxing property that the server runtime uses (doc 01).

## Host Adapter Specification

Any platform that wants to run modules must implement the **Host Adapter** — the set of host functions backing the WIT interfaces. The specification defines the contract:

### Required Behaviours

**`csn::request`** (Consensus)
- MUST forward the JSON-RPC request to a provider for the given chain.
- MUST return the JSON-encoded result (the `result` field from the JSON-RPC response).
- MUST return `json-rpc-error` for provider errors, method-not-found, and transport failures.
- SHOULD enforce a method allowlist (configurable by the operator/user).
- MAY apply middleware (timeout, retry, rate-limit, fallback) — this is platform-specific.

**`identity::accounts/sign/sign-typed-data`** (Identity)
- `accounts` MUST return the list of available account identifiers (addresses) for the current host configuration.
- `sign` MUST produce a valid cryptographic signature over the provided data using the specified account's private key.
- `sign-typed-data` MUST produce a valid EIP-712 signature over the provided typed data structure.
- MUST return `identity-error` if the account is unknown, the user rejects the signing request, or the backend is unavailable.
- MAY prompt the user for approval before signing (platform-dependent — e.g. wallet extension popup in WebView, biometric prompt on mobile).
- SHOULD NOT expose private key material to the module. The module sends data in, gets a signature out.

**`local-store::get/set/delete/list-keys`**
- MUST provide per-module isolation (module A cannot read module B's state).
- MUST persist across module restarts within the same host process/session.
- SHOULD persist across host process restarts (platform-dependent).
- MAY enforce size quotas. If exceeded, `set` returns `Err` (not a trap).
- MAY provide transactional semantics. Modules SHOULD NOT rely on this across platforms.

**`remote-store::upload/download/feed-get/feed-set`**
- MUST route to a Swarm-compatible node or gateway.
- `upload` MUST return the 32-byte content reference of the stored data.
- `download` MUST return the raw bytes for a valid reference, or error for missing/unreachable content.
- `feed-set` signs with the host's identity. The owner is implicit.
- MAY return errors for unavailable connectivity (offline, no node configured).

**`msg::publish/query`**
- MUST route `publish` to a Waku-compatible node.
- `publish` MUST deliver the message to the content topic's relay network on a best-effort basis.
- `query` SHOULD return historical messages if the host's Waku node supports the store protocol.
- `query` MAY return an empty list or error if store is unavailable.
- MAY apply rate limits to prevent message spam.

**`logging::log`**
- MUST accept log calls without blocking or erroring.
- MAY discard logs (e.g. below a configured level threshold).
- Output destination is entirely host-specific.

**Event dispatch (`on-event`)**
- MUST call `init(config)` exactly once before any `on-event` calls.
- MUST call `on-event` for each subscribed event (per manifest).
- MUST support all four event types: `block`, `logs`, `timer`, `message`.
- SHOULD guarantee in-order delivery within a single module.
- MAY dispatch events concurrently across modules.
- SHOULD handle panics/traps gracefully (restart module, not crash host).

### Optional Behaviours (Platform-Specific)

| Capability | Server | Mobile | WebView |
|------------|--------|--------|---------|
| Fuel metering | Yes (wasmtime) | Maybe (engine-dependent) | No (browser engine) |
| Epoch interruption | Yes (Tokio task) | No | No (browser manages scheduling) |
| Memory limits | Yes (`ResourceLimiter`) | Limited (engine-dependent) | No (browser enforces its own limits) |
| Transactional state | Yes (redb write txn) | Optional (SQLite txn) | No (IndexedDB is async) |
| WebSocket subscriptions | Yes | Limited (background constraints) | Yes (if tab is active) |
| Push-based events | N/A | Yes (FCM/APNs) | N/A |
| Waku full relay | Yes | No (light client) | Maybe (js-waku) |

## Content-Addressed Distribution: Works Everywhere

The packaging and distribution model (doc 02, 03) is already platform-agnostic:

```
Module author:
  1. Build WASM component
  2. Create manifest (nexum.toml)
  3. Upload bundle to Swarm -> get content hash
  4. Set ENS contenthash -> content hash

Any host (server, mobile, WebView):
  1. Resolve ENS name -> contenthash
  2. Fetch bundle from Swarm (or IPFS/OCI/HTTP gateway)
  3. Verify sha256(module.wasm) matches manifest
  4. Load module
```

The only platform-specific part is **how** the host fetches from Swarm:
- Server: direct Bee API
- Mobile: Bee gateway over HTTP
- WebView: fetch API to Bee gateway

The content hash is the trust anchor. The transport is interchangeable.

## SDK Layering

The SDK mirrors the WIT layering. nexum ships `nexum-sdk`; downstream distributions may ship their own SDK that re-exports `nexum-sdk` and adds domain-specific wrappers:

```mermaid
graph TD
    subgraph Downstream["Downstream SDK (example: shepherd-sdk)"]
        DS_ITEMS["Domain wrappers (e.g. CowClient, order helpers),\n#[shepherd::module] macro\n(imports cow + order)"]
    end

    subgraph NexumSDK["nexum-sdk (Universal: any blockchain app)"]
        NEXUM_ITEMS["HostTransport, provider(),\nTypedState, RemoteStore,\nMsgClient, Identity,\nlogging macros,\nerror types,\n#[nexum::module] macro\n(imports csn + identity\n+ local-store\n+ remote-store + msg\n+ logging)"]
    end

    Downstream -->|"re-exports + extends"| NexumSDK
```

- **`nexum-sdk`** — the universal Rust SDK for any module targeting `web3:runtime/headless-module`. Provides `HostTransport` (alloy `Transport` trait over `csn::request`), `provider(chain_id)`, `TypedState` (serde over `local-store`), `RemoteStore` (typed wrapper over `remote-store`), `MsgClient` (typed wrapper over `msg`), `Identity` (typed wrapper over `identity`), logging macros, error types. Any module author — CoW, DeFi, gaming, whatever — uses this.

- **Downstream SDKs** — as an example, `shepherd-sdk` extends `nexum-sdk` with CoW-specific wrappers: `CowClient`, order submission helpers, the `#[shepherd::module]` proc macro (which generates `cow` and `order` imports in addition to the universals).

A module author building a generic blockchain automation module depends only on `nexum-sdk`. A module author building a CoW Protocol module depends on Shepherd's `shepherd-sdk`, which re-exports `nexum-sdk`.

For **non-Rust** module authors (JavaScript, Python, Go, C++), the SDK is unnecessary — they use `wit-bindgen` directly against the WIT package for their target world. The WIT is the universal contract; the SDK is a Rust ergonomics layer on top.

## Summary

### Primitive Taxonomy

| Primitive | Interface | Implementation | Persistence | Scope |
|-----------|-----------|---------------|-------------|-------|
| Consensus | `csn` | JSON-RPC (eth_*) | Blockchain | Global (chain) |
| Identity | `identity` | Keystore / KMS / HSM | Key material | Per-account |
| Local Store | `local-store` | redb / SQLite / IndexedDB | Device-local | Per-module |
| Remote Store | `remote-store` | Ethereum Swarm | Decentralised | Global (content-addressed) |
| Messaging | `msg` | Waku | Ephemeral | Topic-based pub/sub |
| Logging | `logging` | tracing / console | None | Diagnostic |

### Architecture

| Concept | Scope |
|---------|-------|
| `web3:runtime` WIT package | Universal — any blockchain app, any platform (ships with nexum) |
| `headless-module` world | Automation modules — server, mobile, background |
| `app-module` world | Interactive modules — WebView, super app |
| Downstream WIT packages | Domain extensions (e.g. `shepherd:cow` for CoW Protocol automation, maintained in downstream repositories) |
| `nexum-sdk` crate | Universal Rust SDK (HostTransport, TypedState, RemoteStore, MsgClient, Identity) |
| Downstream SDK crates | Domain-specific Rust SDKs (e.g. `shepherd-sdk` for CoW, extending `nexum-sdk`) |
| Content-addressed distribution | Platform-agnostic (Swarm/IPFS, ENS discovery, hash verification) |
| Host Adapter | Platform-specific implementation of universal interfaces |

The module binary is the portable artifact. The WIT contract is the universal interface. The host adapter is the platform-specific implementation. Everything else — packaging, distribution, discovery, SDK — layers cleanly on top.
