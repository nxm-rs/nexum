# Module Lifecycle, Event System & Packaging

## Module Package: the Nexum Module Bundle

A module is distributed as a **bundle** — a WASM component plus a manifest that declares its identity, event subscriptions, chain requirements, and resource limits. The manifest is the bridge between packaging, the event system, and the runtime lifecycle.

### Manifest (`nexum.toml`)

Every module ships with a manifest:

```toml
[module]
name = "block-logger"
version = "0.2.0"
description = "Logs new blocks as they arrive"
authors = ["alice.eth"]

# Content hash of the compiled .wasm component
wasm = "sha256:9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08"

[module.resources]
max_memory_bytes = 10_485_760   # 10 MB
max_fuel_per_event = 100_000
max_state_bytes = 52_428_800    # 50 MB

[module.restart]
max_consecutive_failures = 10   # Dead after this many consecutive failures

# Chain requirements — the runtime provides RPC for these
[chains]
required = [1]                   # Mainnet (must have)
optional = [42161, 100]          # Arbitrum, Gnosis (used if available)

# Event subscriptions — declares what the runtime should feed this module
[[subscribe]]
type = "block"
chain_id = 1

[[subscribe]]
type = "log"
chain_id = 1
address = "0xfdaFc9d1902f4e0b84f65F49f244b32b31013b74"
topics = ["0x…"]                 # example log filter

[[subscribe]]
type = "cron"
schedule = "*/5 * * * *"         # every 5 minutes

# Arbitrary key-value config passed to the module at init
[config]
log_prefix = "block"
min_interval_secs = 120
```

Key design points:

- **`wasm` is a content hash**, not a filename. The runtime resolves it via the content store (see below).
- **`subscribe` blocks are declarative.** The module doesn't set up its own subscriptions imperatively — the runtime reads the manifest and wires up event sources before calling `init`.
- **`resources` are caps**, not requests. The runtime enforces them via wasmtime's `ResourceLimiter` and fuel system.
- **`chains.required`** — if the runtime doesn't have an RPC endpoint for a required chain, the module fails to load (fast, clear error).
- **`config`** is opaque to the runtime. All values are **stringified** before being passed to the module's `init` export as `list<tuple<string, string>>` (e.g. TOML integer `120` becomes the string `"120"`). Modules are responsible for parsing values from strings.

### Bundle Format

A bundle is a **directory** with a fixed layout:

```
block-logger/
├── nexum.toml             # manifest
└── module.wasm            # compiled component (matches wasm hash)
```

The runtime validates that `sha256(module.wasm)` matches the hash in the manifest's `wasm` field (after stripping the `sha256:` scheme prefix). This integrity check applies regardless of transport.

How the directory is represented depends on the content backend:

| Backend | Bundle representation |
|---------|---------------------|
| Local filesystem | Directory on disk |
| Swarm | Swarm directory manifest (native directory support) |
| IPFS | UnixFS directory |
| OCI | OCI artifact with manifest + wasm layer |
| HTTPS | tar.gz archive (extracted after download) |

## Content-Addressed Distribution

Distribution is **agnostic** — the runtime resolves content by hash through pluggable backends. The manifest's `wasm` field is a content address; the `source` in the runtime config tells the runtime *where* to look.

### Content Reference Scheme

```
<scheme>:<hash>
```

| Scheme | Example | Resolution |
|--------|---------|------------|
| `sha256` | `sha256:9f86d08…` | Local content store lookup |
| `bzz` | `bzz:22cbb9cedc…` | Ethereum Swarm (64-char hex, 256-bit) |
| `ipfs` | `ipfs:QmYwAPJz…` | IPFS CID |
| `oci` | `oci:ghcr.io/org/block-logger:0.2.0` | OCI registry (CNCF WASM artifact format) |
| `https` | `https://example.com/module.wasm` | Direct HTTP fetch (hash-verified after download) |

### Runtime Content Store

The runtime maintains a local content-addressed store (a directory of blobs keyed by hash). Resolution flow:

```mermaid
flowchart TD
    A[Read manifest] --> B[Extract wasm content reference]
    B --> C{Hash in local store?}
    C -->|Hit| F[Return path to verified .wasm]
    C -->|Miss| D[Resolve via configured backend]
    D --> E[Verify hash of fetched bytes]
    E --> G[Store locally for future use]
    G --> F
```

### Runtime Source Configuration

The operator configures available backends in the runtime config:

```toml
[[content.sources]]
type = "local"
path = "/var/nexum/modules"

[[content.sources]]
type = "swarm"
bee_api = "http://localhost:1633"

[[content.sources]]
type = "oci"
registry = "ghcr.io"

# Sources are tried in order; first match wins.
```

This means:
- A **local dev** just drops `.wasm` files in a directory.
- A **production deployment** fetches from Swarm or OCI on first load, then caches locally.
- **Integrity is always verified** — the content hash in the manifest is the trust anchor, not the transport.

## Module Lifecycle

```mermaid
stateDiagram-v2
    [*] --> Resolve: Fetch wasm by content hash
    Resolve --> Load: Success
    Resolve --> Dead: Failure
    Load --> Init: Component compiled, WIT world validated
    Load --> Dead: Failure
    Init --> Run: init() succeeded
    Init --> Restart: Transient failure
    Run --> Restart: Error / crash
    Restart --> Run: Recovered (backoff: 1s → 2s → 4s → … → 5min cap)
    Restart --> Dead: N consecutive failures (poison pill)
    Run --> Dead: Operator shutdown
    Dead --> [*]
```

### States

| State | Description |
|-------|-------------|
| **Resolve** | Content store resolves `wasm` hash to local path. Fail -> `Dead`. |
| **Load** | `Component::from_file`, create `InstancePre`. Validates that the component satisfies the target WIT world (`web3:runtime/headless-module`, or any downstream world that `include`s it). Fail -> `Dead`. |
| **Init** | Create `Store`, instantiate, call `init(config)` inside an implicit write transaction (same semantics as `on_event` — commit on success, rollback on failure). Module sets up internal state. Fail -> `Restart` (might be transient). |
| **Run** | Runtime dispatches events to `on_event`. Each call gets a fuel budget. Module processes events and may call host imports (csn, local-store, identity). |
| **Restart** | After a trap or error. Backoff: 1s -> 2s -> 4s -> ... -> 5min cap. A fresh `Store` is created (clean memory), but **local-store data persists** (it's in redb, external to the WASM instance). |
| **Dead** | After N consecutive failures (poison pill detection) or explicit operator shutdown. No further event dispatch. Requires manual intervention. |

### Key Lifecycle Properties

- **State survives restarts.** The redb key-value store is external to the WASM instance. A restarted module picks up where it left off.
- **Memory does not survive restarts.** Each restart creates a fresh `Store` — clean linear memory, no stale pointers.
- **`InstancePre` is reused.** Compilation and linking are done once at Load. Restarts only create a new `Store` and call `init` again.
- **Config is immutable for a loaded module.** Changing config requires a reload (new Load cycle).
- **Hot-reload sequence.** When a module update is detected (e.g. ENS contenthash changed): (1) let the current in-flight `on_event` complete, (2) stop event dispatch for this module, (3) fetch and compile the new `Component`, (4) create new `InstancePre`, (5) create fresh `Store`, (6) call `init` with new config — state table is inherited (module handles migration), (7) resume event dispatch. The old `InstancePre` is dropped.

## Event System

### Architecture

```mermaid
flowchart TD
    subgraph ESM["Event Source Manager"]
        BS["Block Subscriber"]
        LW["Log Watcher"]
        CT["Cron Ticker"]
    end

    subgraph NR["Nexum Runtime"]
        ESM
        ER["Event Router\n(manifest subscriptions → modules)"]
        MA["Module A"]
        MB["Module B"]
        MC["Module C"]
    end

    BS --> ER
    LW --> ER
    CT --> ER
    ER --> MA
    ER --> MB
    ER --> MC
```

### Event Sources

| Source | Trigger | Backed by |
|--------|---------|-----------|
| `block` | New block on a chain | `eth_subscribe("newHeads")` via alloy `Provider` |
| `log` | Matching log emitted | `eth_subscribe("logs", filter)` via alloy |
| `cron` | Schedule fires | Tokio `interval` / cron expression parser |

Event sources are **shared**. If two modules subscribe to blocks on chain 1, the runtime maintains a single block subscription and fans out to both.

### Event Router

The router is the core dispatch table. Built at load time from all module manifests:

```rust
struct EventRouter {
    /// block subscriptions: chain_id → [ModuleHandle]
    block_subs: HashMap<u64, Vec<ModuleHandle>>,
    /// log subscriptions: (chain_id, filter_key) → [ModuleHandle]
    log_subs: HashMap<(u64, LogFilterKey), Vec<ModuleHandle>>,
    /// cron subscriptions: schedule → [ModuleHandle]
    cron_subs: Vec<(CronSchedule, ModuleHandle)>,
}
```

When an event fires:

1. Router looks up which modules are subscribed.
2. For each module, spawns a Tokio task (or uses a task pool) to call `on_event`.
3. Each call gets its own fuel budget from the manifest's `max_fuel_per_event`.
4. If the call traps (fuel exhaustion, panic, etc.), the module enters the Restart path.

### Dispatch Semantics

- **Concurrent across modules.** A block event is dispatched to all subscribed modules concurrently. One slow module does not block another.
- **Sequential within a module.** Events for the same module are dispatched in order. A module sees block N before block N+1. This is enforced by a per-module dispatch queue (Tokio `mpsc` channel).
- **Best-effort delivery.** If a module is in Restart state when an event arrives, the event is queued (bounded buffer). If the buffer fills, oldest events are dropped and a warning is logged.
- **No acknowledgement.** A successful return from `on_event` is not an ack. The module is responsible for using the local-store to track its own progress (e.g. "last processed block").
- **Catch-up after gaps.** Events can be dropped during restart (bounded buffer overflow). Modules should query for missed data on startup — e.g. in `init`, read `last_block` from local-store, use the alloy `Provider` (backed by `csn::request`) to call `get_block_number()` and `get_logs()` to backfill any gap. This is a best practice, not enforced by the runtime.

### Event Type Encoding

Events cross the WASM boundary as the `event` variant defined in the WIT:

```wit
variant event {
    block(block-data),
    logs(list<log-entry>),
    timer(u64),
}

record block-data {
    chain-id: u64,
    number: u64,
    hash: list<u8>,
    timestamp: u64,
}
```

The runtime serialises event data via the canonical ABI (handled automatically by `bindgen!`).

## Universal WIT World

The initial WIT in `01-runtime-environment.md` is extended to support the lifecycle and config. Nexum ships a single universal package, `web3:runtime`; downstream distributions can layer their own packages on top.

### Universal Package: `web3:runtime@0.1.0`

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

    variant event {
        block(block-data),
        logs(list<log-entry>),
        timer(u64),
    }

    /// Opaque config map from nexum.toml [config] section.
    type config = list<tuple<string, string>>;
}

interface csn {
    use types.{chain-id};

    record json-rpc-error {
        code: s64,
        message: string,
        data: option<string>,
    }

    /// Generic JSON-RPC passthrough. See doc 07 for full design rationale.
    request: func(chain-id: chain-id, method: string, params: string)
        -> result<string, json-rpc-error>;
}

interface local-store {
    get: func(key: string) -> result<option<list<u8>>, string>;
    set: func(key: string, value: list<u8>) -> result<_, string>;
    delete: func(key: string) -> result<_, string>;
    list-keys: func(prefix: string) -> result<list<string>, string>;
}

interface logging {
    enum level { trace, debug, info, warn, error }
    log: func(level: level, message: string);
}

interface identity {
    record identity-error { code: u16, message: string }
    accounts: func() -> result<list<list<u8>>, identity-error>;
    sign: func(account: list<u8>, data: list<u8>) -> result<list<u8>, identity-error>;
    sign-typed-data: func(account: list<u8>, typed-data: string) -> result<list<u8>, identity-error>;
}

/// Universal headless module world — platform-agnostic.
world headless-module {
    import csn;
    import local-store;
    import logging;
    import identity;

    /// Called once on load. Receives config from nexum.toml.
    export init: func(config: types.config) -> result<_, string>;

    /// Called for each subscribed event.
    export on-event: func(event: types.event) -> result<_, string>;
}
```

### Downstream extensions

Third-party distributions may layer their own WIT packages on top. As an example, the Shepherd distribution defines a `shepherd:cow` package that adds CoW Protocol REST and order-submission interfaces on top of the universal world via WIT `include`. That extension is not part of nexum itself — it lives in the downstream repository that builds on nexum.

## Putting It All Together

Operator deploys a module:

```
1. Operator adds entry to runtime config:

   [[modules]]
   manifest = "/var/nexum/block-logger/nexum.toml"

2. Runtime reads manifest:
   - Resolves wasm content hash → fetches from Swarm/local/OCI
   - Verifies integrity (sha256 match)

3. Runtime compiles Component, creates InstancePre:
   - Validates component satisfies target world
     (web3:runtime/headless-module, or a downstream world
      that includes it)
   - Enforces resource limits from manifest

4. Runtime calls init(config):
   - Module receives [config] section as key-value pairs
   - Module sets up internal state, logs readiness

5. Runtime wires event sources from [[subscribe]] blocks:
   - Creates/reuses block subscriber for chain 1
   - Creates log watcher with address + topic filter
   - Registers cron schedule

6. Events flow:
   Block 19_000_001 on Mainnet
   → Router → block-logger's dispatch queue
   → Tokio task calls on_event(Event::Block(…))
   → Module calls csn::request (via alloy Provider) and local_store_get
   → Returns Ok(()) — runtime logs success

7. On crash:
   → Module trapped (fuel exhaustion / panic)
   → Runtime logs error, enters Restart state
   → Backoff 1s, creates fresh Store, calls init again
   → Local-store data still intact — module resumes
```
