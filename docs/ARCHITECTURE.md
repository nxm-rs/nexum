# Nexum Architecture

This document provides a comprehensive overview of Nexum's architecture, component organization, and design patterns.

## Overview

Nexum is a high-performance Ethereum provider written in Rust and compiled to WebAssembly. It provides both a Chrome browser extension and a terminal-based interface (TUI) for interacting with Ethereum networks. The codebase is organized into multiple focused crates that handle different concerns: APDU smart card operations, Keycard integration, core RPC functionality, and extension components.

**Key Technologies:**
- Language: Rust (2024 edition, MSRV 1.88)
- Platforms: WebAssembly (browser), native (terminal)
- License: AGPL-3.0-or-later
- Primary dependencies: Alloy (Ethereum library), jsonrpsee (JSON-RPC), tokio (async runtime)

---

## Workspace Structure

The project uses a Cargo workspace with the following member organization:

```
nexum/
├── crates/
│   ├── apdu/                       # Smart card APDU protocol layer
│   │   ├── core/                   # Core APDU types and traits
│   │   ├── globalplatform/         # GlobalPlatform applet commands
│   │   ├── macros/                 # APDU code generation macros
│   │   └── pcsc/                   # PC/SC transport (physical card readers)
│   │
│   ├── keycard/                    # Keycard (smart card) integration
│   │   ├── keycard/                # Core Keycard protocol and operations
│   │   ├── signer/                 # Ethereum transaction signing with Keycard
│   │   └── cli/                    # Keycard command-line utility
│   │
│   └── nexum/                      # Main Nexum components
│       ├── primitives/             # Shared types and protocols (WASM)
│       ├── rpc/                    # JSON-RPC server and namespaces
│       ├── tui/                    # Terminal user interface (ratatui)
│       └── extension/              # Browser extension components
│           ├── chrome-sys/         # Chrome API bindings (WASM)
│           ├── worker/             # Service worker (WASM)
│           ├── injector/           # Content script injector (WASM)
│           ├── injected/           # Injected frame script (WASM)
│           └── browser-ui/         # Extension popup UI (Leptos/WASM)
```

**Default members for `cargo build`:** `rpc` and `tui` (the CLI applications)

---

## Core Components

### 1. APDU Layer (`crates/apdu/`)

**Purpose:** Abstraction over Application Protocol Data Unit (APDU) communication with smart cards following ISO/IEC 7816-4 standard.

#### `apdu/core` - Foundation
- **Main types:**
  - `ApduCommand`: Represents APDU commands (CLA, INS, P1, P2, data, expected length)
  - `ApduResponse`: Represents card responses with payload and status word
  - `StatusWord`: ISO status codes (e.g., 0x9000 = success)

- **Key abstractions:**
  - `CardTransport`: Trait for different transport mechanisms (PC/SC, mock, etc.)
  - `CommandProcessor`: Pipeline for transforming commands (e.g., get response handling)
  - `Executor`: Execute commands with optional secure channel wrapping
  - `SecureChannel`: Encrypted/MAC'd communication with cards
  - `ProcessorPipeline`: Chain multiple processors for complex workflows

- **Features:**
  - `std` (default): Enables error types and features requiring std
  - `longer_payloads`: Support for extended APDU with larger payloads

#### `apdu/globalplatform` - Applet Management
- Commands for GlobalPlatform-compliant smart cards
- Application lifecycle management (install, delete, select)
- Used to interact with Keycard applets

#### `apdu/pcsc` - PC/SC Transport
- Physical connection to smart card readers via PC/SC interface
- `PcscTransport`: Low-level transport implementation
- `PcscDeviceManager`: Manages available card readers
- Supports event-driven card detection and connection monitoring

#### `apdu/macros` - Code Generation
- Procedural macros to reduce boilerplate for APDU command builders
- Generates serialization/deserialization code for command structures

### 2. Keycard Layer (`crates/keycard/`)

**Purpose:** High-level Ethereum-specific smart card operations using Keycard protocol.

#### `keycard/keycard` - Core Keycard Protocol
- **Main type:** `Keycard<T>` - Generic over transport type
- **Key functionalities:**
  - Secure channel establishment and cryptographic operations
  - Derive and manage Ethereum keypairs
  - Sign transactions and messages with BIP32 hierarchical determinism
  - PIN/PUK management for card security
  - Pairing for multi-device trust

- **Dependencies on APDU:**
  - Uses `CardExecutor` from apdu-core for APDU command execution
  - Implements secure channel wrapping using ISO 7816-4 mechanisms
  - Crypto: AES-128, CBC-MAC for secure channels; PBKDF2 for key derivation

#### `keycard/signer` - Ethereum Signer Integration
- Implements `alloy::signers::Signer` trait for seamless integration with Alloy
- Provides async transaction signing with Keycard
- Supports EIP-712 typed data signing
- Used by both CLI tools and RPC server when signing is needed

#### `keycard/cli` - Command-Line Tool
- Utility for direct Keycard operations
- Commands for PIN management, key derivation, transaction signing
- Useful for Keycard initialization and management

### 3. Nexum Core (`crates/nexum/`)

#### `primitives` - Shared Types (WASM-compatible)
- **File structure:**
  - `frame.rs`: `ConnectionState`, `FrameState` - tracks extension connection and chain info
  - `protocol.rs`: Message types for inter-component communication

- **Key types:**
  - `ProtocolMessage`: Wrapper around `MessageType` with "nexum" protocol identifier
  - `MessageType`: Enum for EthEvent, Request, Response variants
  - `RequestWithId`, `ResponseWithId`: JSON-RPC 2.0 style request/response with IDs
  - Conversion functions between JsValue and Rust types (via serde-wasm-bindgen)

- **Purpose:** Single source of truth for types shared between extension components and with web pages

#### `rpc` - JSON-RPC Server
- **Components:**
  - `RpcServerBuilder`: Configures and builds the server with chains and providers
  - `namespaces/`:
    - `eth.rs`: Ethereum namespace (account management, signing, sending transactions)
    - `net.rs`: Network utilities
    - `wallet.rs`: Wallet-specific methods
    - `web3.rs`: Web3 utilities

- **Architecture:**
  - Built on `jsonrpsee` (0.24.7) with hyper/tokio for HTTP/WS
  - Middleware support for custom logic (CallerContext)
  - Interactive request handling: `enum InteractiveRequest` for user-facing operations
  - Fillers pattern from Alloy: `ChainIdFiller`, `GasFiller`, `NonceFiller`, `BlobGasFiller`

- **Key entry points:**
  - `RpcServerBuilder::new()` - Create builder
  - `.chain(NamedChain, Url)` - Add blockchain RPC endpoint
  - `.build()` - Returns `(ServerHandle, Receiver<InteractiveRequest>)`
  - Interactive requests like `EthRequestAccounts`, `EthSignTransaction` are sent to the client for user confirmation

#### `tui` - Terminal User Interface
- **Technology:** ratatui (TUI framework), crossterm (cross-platform terminal control), tokio
- **Architecture:**
  - Main event loop: `EventStream` for keyboard/terminal events
  - Tab-based UI with multiple views (config, signers, etc.)
  - Integration with `RpcServer` for receiving interactive requests
  - `nexum_rpc::RpcServerBuilder` creates server listening on configurable host:port

- **Key components:**
  - `Config`: TOML-based configuration management
  - `signers`: Account management (Ledger support via `load_ledger_accounts()`)
  - Response channel: Sends `InteractiveResponse` back to RPC server for request fulfillment
  - Logging to file (`nxm.log`) via custom writer

---

## Extension Architecture (`crates/nexum/extension/`)

Browser extension is built with multiple WASM components coordinating via Chrome APIs and message passing.

### Extension Flow Diagram
```
Web Page
   ↓
[injected.js] ← frame script (WASM, eip6963/provider)
   ↓ (message event)
[injector.js] ← content script (WASM, message relay)
   ↓ (chrome.runtime.sendMessage)
[worker.js] ← service worker (WASM, state/RPC relay)
   ↓ (chrome.runtime.connect)
[browser-ui/index.html] ← popup panel (Leptos/WASM)
   ↓
[upstream RPC] (ws://127.0.0.1:1250/...)
```

### Component Details

#### `chrome-sys` - Chrome API Bindings
- **Modules:**
  - `runtime.rs`: `addOnMessageListener()`, `sendMessage()`, `getURL()` - extension messaging
  - `tabs.rs`: `query()`, `get_active_tab()`, `send_message_to_tab()` - tab management
  - `port.rs`: Long-lived message ports for popup↔worker communication
  - `alarms.rs`: Periodic alarms for status checks
  - `idle.rs`: Detect browser idle state
  - `action.rs`: Set extension icon/popup

- **Pattern:** Thin WASM-bindgen wrappers around Chrome extension APIs, using futures for async operations

#### `worker` - Service Worker (Background Script)
- **Entry point:** `initialize_extension()` - called from manifest's service worker script
- **Architecture:**
  - Builder pattern: `ExtensionBuilder` → `Extension` struct
  - Manages `ExtensionState` (tab origins, connection state)
  - Maintains `Provider` for RPC communication

- **Modules:**
  - `provider.rs`: JSON-RPC client connection to upstream (hardcoded: `ws://127.0.0.1:1250/sepolia`)
    - Methods: `init()`, `reset()`, `verify_connection()`, `on_connect()`, `on_disconnect()`
    - Monitoring loop with periodic heartbeat
  - `state.rs`: Extension state management
  - `events/`: Handles Chrome API events
    - `runtime.rs`: Background script lifecycle events
    - `tabs.rs`: Tab activation/removal
    - `idle.rs`: Browser idle state changes
    - `alarm.rs`: Periodic status checks
  - `subscription.rs`: Event subscription logic

- **Key constants:**
  - `EXTENSION_PORT_NAME = "frame_connect"` - for popup communication
  - `CLIENT_STATUS_ALARM_KEY = "check-client-status"` - periodic heartbeat (0.5 min)
  - `UPSTREAM_URL = "ws://127.0.0.1:1250/sepolia"` - hardcoded RPC endpoint

#### `injector` - Content Script (Injector)
- **Entry point:** `#[wasm_bindgen(start)] run()`
- **Responsibilities:**
  1. Runs in content script context (has access to both page and extension)
  2. Injects `injected.js` as a `<script>` tag into the page
  3. Sets up event listener for `chrome.runtime.onMessage` events
  4. Relays page → extension messages via `chrome_sys::runtime::send_message()`
  5. Relays extension → page messages via `window.postMessage()`

- **Flow:**
  - Listen for `message` events from page context
  - Validate payload is `ProtocolMessage`
  - Forward to extension via Chrome runtime
  - Receive responses from extension via runtime listener
  - Post back to page via `window.postMessage()`

#### `injected` - Injected Frame Script
- **Entry point:** `initialize_provider()` - called from script tag
- **Responsibilities:**
  1. Runs in page's window context (same origin policy)
  2. Creates `EthereumProvider` instance
  3. Exposes provider as `window.ethereum`
  4. Implements EIP-1193 and EIP-6963 standards

- **Key behaviors:**
  - Handles existing `window.ethereum` (respects configurable property descriptor)
  - Uses reflection API to safely set properties
  - Communicates with injector via `window.postMessage()` / message events

- **Provider implementation:**
  - `eip6963.rs`: Announces provider via EIP-6963 event
  - `provider.rs`: EthereumProvider struct implementing RPC method routing

#### `browser-ui` - Extension Popup UI
- **Technology:** Leptos (reactive web framework), compiled to WASM
- **Entry point:** `#[component] App()` - root component
- **Architecture:**
  - Leptos signals for reactive state: `active_tab`, `frame_state`, `mm_appear`, `is_injected_tab`
  - Connects to worker via `chrome_sys::runtime::connect()` using `EXTENSION_PORT_NAME`
  - Periodic polling (1-second interval) via `send_message_to_tab()` to get chain ID

- **Modules:**
  - `pages/`: Page components (Settings page is primary)
  - `panels/`: Conditional panels (Connected, Not Connected, Unsupported Tab)
  - `components/`: Reusable UI components (buttons, logos, overlays)
  - `helper.rs`: Utility functions
  - `constants.rs`: Configuration values

- **State management:**
  - `FrameState`: Tracks frame connection status and available chains per tab
  - Communicates with background worker for persistent state

---

## Communication Protocols

### Inter-Component Message Format
All inter-component communication uses the `ProtocolMessage` structure (from primitives):
```rust
{
  "protocol": "nexum",
  "message": {
    "EthEvent": { "event": "...", "args": [...] } |
    "Request": { "id": "...", "method": "...", "params": [...] } |
    "Response": { "id": "...", "result": {...} or "error": {...} }
  }
}
```

### Message Flow Examples

1. **DApp → Extension RPC Call:**
   - Page calls `window.ethereum.request({ method: "eth_accounts" })`
   - Injected script sends Request via ProtocolMessage
   - Injector relays to worker
   - Worker sends to upstream RPC (1250)
   - Response flows back through same path

2. **Provider Status Update:**
   - Worker checks connection health (alarms, idle detection)
   - Emits `EthEvent` with connection state change
   - Browser-UI receives via port message listener
   - UI re-renders with new connection state

---

## Build Configuration

### WASM Targets
- **Default:** `cdylib` (dynamic library for WASM)
- **Profile:** Release builds use:
  - LTO (Link-Time Optimization)
  - `opt-level = "z"` (optimize for size)
  - `wasm-opt = false` in wasm-pack config (disables wasm-opt postprocessing)

### Compilation
```bash
# Default (RPC + TUI)
cargo build --release

# WASM components
wasm-pack build -t web --release crates/nexum/extension/worker
wasm-pack build -t web --release crates/nexum/extension/injector
wasm-pack build -t web --release crates/nexum/extension/injected
# Note: browser-ui uses leptos-specific build (handled by Leptos tooling)
```

---

## Key Design Patterns

### 1. Builder Pattern
- `ExtensionBuilder` for Extension initialization
- `RpcServerBuilder` for RPC server configuration
- Fluent API for composable configuration

### 2. Generic Over Transport
- `Keycard<T: CardExecutor>` allows different transport backends (PC/SC, mock, etc.)
- `Provider<T: CardTransport>` in APDU layer
- Facilitates testing and swapping implementations

### 3. Async-First
- Tokio runtime for blocking operations
- `wasm_bindgen_futures` for WASM-compatible async
- Futures-based APIs throughout (`.await` syntax)

### 4. Error Handling
- `Result<T>` type aliases return custom `Error` enum
- `thiserror` for ergonomic error definitions
- Error context via `ResultExt` trait (eyre integration)
- Panic hooks for WASM debugging

### 5. State Management
- `Arc<Mutex<T>>` for shared mutable state in WASM (async-lock crate)
- Atomic types (`AtomicBool`) for simple flags
- Immutable types for primitives/protocol definitions

### 6. Tracing & Observability
- `tracing` crate for structured logging
- `wasm-tracing` for WASM console output
- Span-based context tracking
- Log output to file in TUI (`nxm.log`)

---

## Entry Points

### Native Applications
1. **TUI:** `crates/nexum/tui/src/main.rs`
   - Entrypoint: `#[tokio::main] async fn main()`
   - Initializes ratatui terminal, spawns RPC server, runs event loop

2. **RPC Server:** Invoked by TUI via `RpcServerBuilder::new().build().await`
   - Listens on configurable host:port (default: 127.0.0.1:1248)
   - Serves HTTP and WebSocket JSON-RPC endpoints

3. **Keycard CLI:** `crates/keycard/cli/src/` (CLI utility for Keycard operations)

### WASM/Extension
1. **Worker:** Called from manifest background script
   - Entrypoint: `worker.initialize_extension()`
   - Async initialization, spawns event listeners

2. **Injector:** Auto-runs on all pages (content script)
   - Entrypoint: `#[wasm_bindgen(start)] run()`
   - Injects provider script

3. **Injected:** Injected into page context
   - Entrypoint: `initialize_provider()`
   - Must be called after script loads

4. **Browser-UI:** Popup panel
   - Entrypoint: Leptos App component
   - Auto-rendered in popup context

---

## Configuration & Deployment

### Extension Manifest (Manifest V3)
- Located: `crates/nexum/extension/public/manifest.json`
- Service worker: `worker.js` (compiled WASM module)
- Content script: `injector.js` (compiled WASM module)
- Action popup: `index.html` (browser-ui output)
- Requires Chrome 116+
- Permissions: tabs, activeTab, alarms, idle, scripting
- Host permissions: http://*/* and https://*/*

### Configuration Files
- **TUI:** TOML-based config in user config directory (figment + toml)
- **Extension:** Hardcoded upstream RPC URLs (ws://127.0.0.1:1250/{chain})
- **Logging:** Configurable via `RUST_LOG` env var (tracing_subscriber)

---

## Testing & Development

### Module Organization for Testing
- APDU tests in `crates/apdu/core/src/lib.rs` (lib reexport tests)
- Keycard tests in `crates/keycard/keycard/src/` (private test modules)
- Integration tests in `crates/apdu/pcsc/tests/`

### WASM-Specific Testing
- `wasm-bindgen-test` for WASM unit tests
- Tests run via `wasm-pack test --headless --firefox`

### Local Development
- RPC server can run standalone: `cargo run -p nexum-rpc -- --host 127.0.0.1 --port 1248`
- TUI requires running RPC server on 1250 for upstream
- Extension popup can be tested with Chrome DevTools in extension page

---

## Dependencies Overview

### Core Runtime
- **tokio:** Async runtime (full features enabled)
- **hyper:** HTTP transport
- **jsonrpsee:** JSON-RPC 2.0 server framework

### Blockchain
- **alloy:** Ethereum library (signers, providers, primitives)
- **alloy-chains:** Chain ID definitions
- **k256:** Secp256k1 curve operations

### Cryptography
- **AES, CBC-MAC:** Smart card secure channels
- **PBKDF2, SHA2:** Key derivation
- **coins-bip39, coins-bip32:** Hierarchical determinism

### WASM-Specific
- **wasm-bindgen:** WASM ↔ JS boundary layer
- **web-sys:** Web platform APIs
- **js-sys:** JavaScript object access
- **gloo-utils:** Utility functions for WASM
- **leptos:** Reactive web framework
- **wasm-tracing:** Logging bridge

### UI
- **ratatui:** Terminal UI framework
- **crossterm:** Cross-platform terminal control
- **styled:** CSS-in-Rust for browser-ui

### Serialization
- **serde, serde_json:** Serialization framework
- **serde-wasm-bindgen:** Serde integration for WASM

---

## Notable Conventions

1. **Module Organization:** `lib.rs` files typically export submodules; `mod.rs` in directories
2. **Error Types:** Dedicated `error.rs` per crate with custom `Error` enum
3. **Async Functions:** `.await` required; futures returned from most functions
4. **WASM Boundaries:** All serializable types implement `Serialize`/`Deserialize`
5. **Workspace Inheritance:** Dependencies, versions, lints defined in root Cargo.toml
6. **Logging:** Span-based tracing with `#[tracing::span]` and `debug!()`, `info!()` macros

---

## Quick Navigation

- **Smart Card Operations:** Start at `crates/apdu/core/src/lib.rs`
- **Keycard Integration:** Check `crates/keycard/keycard/src/lib.rs`
- **RPC Protocol:** Review `crates/nexum/rpc/src/rpc.rs`
- **Extension State:** See `crates/nexum/extension/worker/src/state.rs`
- **Type Definitions:** Look in `crates/nexum/primitives/src/`
- **UI Components:** Navigate `crates/nexum/extension/browser-ui/src/`
- **Manifest & Build:** Check `crates/nexum/extension/public/manifest.json`
