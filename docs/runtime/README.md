# Nexum Runtime Design Documentation

This directory holds the design documentation for the WASM Component Model runtime that ships with nexum — a host-side execution environment that loads guest WASM modules and exposes a set of universal, capability-based interfaces (consensus, identity, local store, remote store, messaging, logging) via WIT.

The documents below are the canonical design source. Downstream protocol-specific extensions — for example, the Shepherd distribution's CoW Protocol module (`shepherd:cow`) — build on top of the primitives described here; they are not part of nexum itself.

1. [00-overview.md](./00-overview.md) — high-level overview, design principles, and the six universal primitives.
2. [01-runtime-environment.md](./01-runtime-environment.md) — wasmtime + Component Model choices and the `web3:runtime/headless-module` world.
3. [02-modules-events-packaging.md](./02-modules-events-packaging.md) — module bundles, manifest format, lifecycle, and event system.
4. [03-module-discovery.md](./03-module-discovery.md) — static, ENS, and on-chain registry discovery.
5. [04-state-store.md](./04-state-store.md) — redb-backed per-module local store with transactional semantics.
6. [05-sdk-design.md](./05-sdk-design.md) — `nexum-sdk` crate design, `#[nexum::module]` macro, and testing framework.
7. [06-production-hardening.md](./06-production-hardening.md) — resource limits, restart policy, metrics, health, and deployment.
8. [07-rpc-namespace-design.md](./07-rpc-namespace-design.md) — generic JSON-RPC passthrough, `HostTransport`, and async module handlers.
9. [08-platform-generalisation.md](./08-platform-generalisation.md) — layered WIT worlds and server / mobile / WebView / super app targets.
