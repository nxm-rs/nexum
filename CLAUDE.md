# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Nexum is a high-performance Ethereum provider written in Rust and compiled to WebAssembly. It provides both a Chrome browser extension and a terminal-based interface (TUI) for interacting with Ethereum networks.

**Key Info:**
- **Language:** Rust (2024 edition, MSRV 1.88)
- **Platforms:** WebAssembly (browser), native (terminal)
- **License:** AGPL-3.0-or-later
- **Primary dependencies:** Alloy (Ethereum), jsonrpsee (JSON-RPC), tokio (async runtime)

## Architecture

**IMPORTANT:** For detailed architecture documentation, see [docs/ARCHITECTURE.md](./docs/ARCHITECTURE.md). When making significant architectural changes or updates, ensure that ARCHITECTURE.md is updated accordingly and those changes are reflected in the relevant sections of this file.

### High-Level Structure

Nexum is organized into three main layers:

1. **APDU Layer** (`crates/apdu/`) - Smart card communication protocol
   - `core`: Foundation types (ApduCommand, ApduResponse, CardTransport, Executor)
   - `globalplatform`: Application lifecycle management
   - `pcsc`: Physical card reader interface
   - `macros`: Code generation for command builders

2. **Keycard Layer** (`crates/keycard/`) - Ethereum-specific smart card operations
   - `keycard`: Core protocol with secure channels, keypair derivation, signing
   - `signer`: Alloy integration for transaction signing
   - `cli`: Command-line utility

3. **Nexum Core** (`crates/nexum/`) - Main application components
   - `primitives`: Shared types (ProtocolMessage, FrameState)
   - `rpc`: JSON-RPC server with namespaces (eth, net, wallet, web3)
   - `tui`: Terminal interface with ratatui
   - `extension`: Five WASM components for browser integration

### Browser Extension Architecture

The extension uses 5 interconnected WASM components:

```
Web Page → [injected.js] → [injector.js] → [worker.js] ⇄ [browser-ui]
                                               ↓
                                          Upstream RPC
```

- **injected.js**: EIP-1193/EIP-6963 provider in web page context
- **injector.js**: Content script that relays messages between page and extension
- **worker.js**: Service worker managing state and upstream RPC connection (ws://127.0.0.1:1250)
- **browser-ui**: Leptos-based popup UI
- **chrome-sys**: Chrome API bindings

All components communicate using `ProtocolMessage` format from `primitives`.

---

## Common Development Commands

### Building

```bash
# Build everything (RPC + TUI)
cargo build --all-targets --all-features --workspace

# Build specific component
cargo build -p tui
cargo build -p nexum-rpc
cargo build -p keycard-cli

# Build browser extension (requires wasm-pack, trunk, just)
just build-ext

# Run extension with live reload (requires web-ext)
just run-ext

# Package extension for distribution
just pack-ext
```

### Testing

```bash
# Run all tests
cargo test --all-targets --all-features --workspace
# Or: just test

# Run tests for specific package
cargo test -p nexum-rpc

# Run a specific test
cargo test test_name

# Run WASM tests (requires wasm-pack)
wasm-pack test --headless --firefox
```

### Linting

```bash
# Run clippy (must pass with no warnings)
cargo clippy --all-targets --all-features --workspace -- -Dwarnings
# Or: just clippy

# Check WASM-specific code
cargo check --target wasm32-unknown-unknown -p worker -p injected -p injector -p browser-ui
```

### Running

```bash
# Run TUI
cargo run -p tui

# Run RPC server standalone
cargo run -p nexum-rpc -- --host 127.0.0.1 --port 1248

# Run Keycard CLI
cargo run -p keycard-cli -- --help
```

---

## Development Guidelines

### Code Quality Standards

- **All code must pass clippy without warnings** - CI enforces this with `-Dwarnings`
- **Tests are required** for new functionality and bug fixes
- **Document public APIs** - Missing docs will trigger warnings
- **Follow workspace lints** - See `Cargo.toml` for configured lints
- **Error handling** - Use appropriate error types, avoid `.unwrap()` in library code

### Testing Requirements

- Unit tests for new functionality
- Integration tests for RPC/extension changes
- WASM tests using `wasm-bindgen-test` where applicable
- Tests should be focused and test one thing

### Commit Messages

Follow conventional commit format:
- `feat: description` - New features
- `fix: description` - Bug fixes
- `docs: description` - Documentation changes
- `refactor: description` - Code refactoring
- `test: description` - Test additions/changes
- `chore: description` - Maintenance tasks

Reference issue numbers when applicable: `fix: resolve connection timeout (#123)`

### AI Assistance Disclosure

**IMPORTANT:** When creating pull requests, you must disclose if AI assistance was used in:
- Code generation or significant completion beyond single-keyword tab-completion
- Documentation or comment writing
- PR descriptions
- Review response composition

**AI disclosure is ONLY required in PR descriptions, NOT in commit messages.**
- Do NOT add "Generated with Claude Code" or similar mentions to commits
- Do NOT add Claude as a co-author in commits
- Keep commit messages clean and follow conventional commit format

See [CONTRIBUTING.md](./CONTRIBUTING.md) for full details on AI assistance disclosure requirements and standards.

---

## Key Technical Details

### Message Protocol

All extension components communicate using `ProtocolMessage` (from `primitives`):
```rust
{
  "protocol": "nexum",
  "message": {
    "EthEvent" | "Request" | "Response": { ... }
  }
}
```

### Design Patterns

1. **Builder Pattern** - Used in `ExtensionBuilder`, `RpcServerBuilder`
2. **Generic Over Transport** - `Keycard<T: CardExecutor>` for different backends
3. **Async-First** - Tokio runtime, `wasm_bindgen_futures` for WASM
4. **Error Handling** - `thiserror` for custom errors, `eyre` for context
5. **State Management** - `Arc<Mutex<T>>` for shared WASM state

### Entry Points

- **TUI:** `crates/nexum/tui/src/main.rs`
- **RPC Server:** `crates/nexum/rpc/src/lib.rs` (invoked by TUI)
- **Keycard CLI:** `crates/keycard/cli/src/`
- **Extension Worker:** `crates/nexum/extension/worker/src/lib.rs::initialize_extension()`
- **Extension Injector:** `crates/nexum/extension/injector/src/lib.rs::run()`
- **Extension Injected:** `crates/nexum/extension/injected/src/lib.rs::initialize_provider()`
- **Extension UI:** `crates/nexum/extension/browser-ui/src/app.rs::App()`

### Important Constants

- **Extension Port:** `"frame_connect"` (worker ↔ popup communication)
- **Upstream RPC:** `ws://127.0.0.1:1250/sepolia` (hardcoded in worker)
- **Status Check Interval:** 0.5 minutes (alarm-based heartbeat)
- **Default RPC Port:** 1248 (TUI's RPC server)

---

## Configuration

### TUI Configuration
- TOML-based config in user config directory
- Uses `figment` for configuration management
- Logging via `RUST_LOG` environment variable
- Logs written to `nxm.log`

### Extension Configuration
- Manifest V3: `crates/nexum/extension/public/manifest.json`
- Requires Chrome 116+
- Permissions: tabs, activeTab, alarms, idle, scripting
- Hardcoded upstream URLs in worker code

---

## Troubleshooting Common Issues

### Build Issues

**WASM build failures:**
- Ensure `wasm-pack` is installed: `cargo install wasm-pack`
- Check Rust target: `rustup target add wasm32-unknown-unknown`
- For browser-ui: Ensure `trunk` is installed: `cargo install trunk`

**Clippy warnings:**
- Fix all warnings before committing - CI will reject code with warnings
- Use `cargo clippy --fix` for auto-fixable issues

**Missing dependencies:**
- Run `cargo fetch` to download all dependencies
- For extension: Install `web-ext` globally: `npm install -g web-ext`

### Runtime Issues

**Extension not connecting to RPC:**
- Verify TUI is running: `cargo run -p tui`
- Check RPC server is listening on port 1250 (not 1248)
- Inspect Chrome DevTools → Service Worker console for errors

**Keycard communication failures:**
- Ensure PC/SC daemon is running on your system
- Check card reader is connected: `cargo run -p keycard-cli -- list`
- Verify Keycard applet is installed on the card

---

## Quick Navigation

For detailed information about specific components, see:

- **Full Architecture:** [docs/ARCHITECTURE.md](./docs/ARCHITECTURE.md)
- **Contributing Guidelines:** [CONTRIBUTING.md](./CONTRIBUTING.md)
- **User Documentation:** [README.md](./README.md)

**Key source locations:**
- Smart card operations: `crates/apdu/core/src/lib.rs`
- Keycard protocol: `crates/keycard/keycard/src/lib.rs`
- RPC server: `crates/nexum/rpc/src/rpc.rs`
- Extension worker: `crates/nexum/extension/worker/src/`
- Shared types: `crates/nexum/primitives/src/`
- TUI: `crates/nexum/tui/src/main.rs`
