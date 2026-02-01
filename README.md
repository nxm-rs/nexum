# Nexum

[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL--3.0-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)

**High-performance Ethereum access. Rust brings web and terminal together for developers who read the docs.**

**Under active development.** Expect changes as we refine features and improve stability. Contributions and feedback welcome.

## What is Nexum?

Nexum is an Ethereum provider written in Rust and compiled to WebAssembly, built for both web extensions and terminal interfaces. Forked from [frame-extension](https://github.com/frame-labs/frame-extension), Nexum is an `EIP-1193`-compliant provider that offers secure, developer-friendly access to Ethereum.

Nexum combines **WebTransport** with a **terminal-based** interface: ideal for developers seeking a flexible, performant tool to interact with Ethereum across web and terminal environments.

## Goals

1. **Compliance**: Full [`EIP-1193`](https://eips.ethereum.org/EIPS/eip-1193) and [`EIP-6963`](https://eips.ethereum.org/EIPS/eip-6963) compliance.
2. **Performance**: Rust and WASM for optimal speed and secure memory management.
3. **Web and Terminal Integration**: Uses [WebTransport](https://developer.mozilla.org/en-US/docs/Web/API/WebTransport) for seamless connection across environments.
4. **Hackable**: Developer-first Ethereum access, suitable for dApps, testing, and experiments.

## Status

Nexum is in **active development**. Documentation, user guides, and installation instructions are in progress. Follow along and contribute as we grow Nexum into a powerful tool for web and terminal-based Ethereum interaction.

## For Users

**Guide coming soon.**

## For Developers

### Using Nexum as a Library

**Crate docs coming soon.**

### Contributing

Nexum welcomes community contributions. To get involved:

- Join the [Matrix space](https://matrix.to/#/#nexum:nxm.rs) to discuss development
- Open an [issue](https://github.com/nxm-rs/nexum/issues) with ideas or questions

### Building and Testing

Clone and build Nexum:

```sh
git clone https://github.com/nxm-rs/nexum
cd nexum
cargo install wasm-pack wasm-opt
wasm-pack build -t web --release -d ../../dist/pkg crates/worker
```

## Licence

[AGPL-3.0-or-later](./LICENSE)
