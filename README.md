# Nexum 🚧⛏️

**🚀 Blazing-fast Ethereum access, where Rust brings web and terminal together for hackers.**

**⚠️ Under Construction ⚠️**  
Nexum is actively being developed! While we’re excited to share progress, expect changes as we refine features and improve stability. Contributions and feedback are welcome as we build.

---

<!--![CI status](https://github.com/your-org/nexum/workflows/CI/badge.svg)-->

<!--![Telegram Chat][tg-badge]-->

<!-- ![](./assets/nexum-banner.png) -->

## What is Nexum?

Nexum is a high-performance Ethereum provider written in Rust and compiled to WebAssembly, built for both web extensions and terminal interfaces. Forked from [frame-extension](https://github.com/frame-labs/frame-extension), Nexum is a `EIP-1193`-compliant provider that offers secure and hacker-friendly access to Ethereum.

Nexum combines **WebTransport** with a **terminal-based** interface, ideal for developers seeking a flexible, performant tool to interact with Ethereum across web and terminal environments.

## Goals 🥅

1. **Compliance**: Full [`EIP-1193`](https://eips.ethereum.org/EIPS/eip-1193) and [`EIP-6963`](https://eips.ethereum.org/EIPS/eip-6963) compliance.
2. **Performance**: Rust and WASM for optimal speed and secure memory management.
3. **Web & Terminal Integration**: Uses [WebTransport](https://developer.mozilla.org/en-US/docs/Web/API/WebTransport) for a seamless connection across environments.
4. **Hackable**: Developer-first Ethereum access, suitable for dApps, testing, and experiments.

## Status 📍

Nexum is in **active development**. Documentation, user guides, and installation instructions are in progress. Follow along and contribute as we grow Nexum into a powerful tool for web and terminal-based Ethereum interaction!

## For Users

**Guide coming soon!** 📖

## For Developers

### Using Nexum as a Library

**Crate docs coming soon!** 📚

### Contributing 🤝

Nexum welcomes community contributions! To get involved:

- Join the [Signal](https://signal.group/#CjQKIHNV-kWphhtnpwS3zywC7LRr5BEW9Q1XyDl2qZtL2WYqEhAyO0c8tGmrQDmEsY15rALt) group to discuss development.
- Open an [issue](https://github.com/nullisxyz/nexum/issues) with ideas or questions.

### Building and Testing 🛠️

Minimum Supported Rust Version (MSRV): [1.82.0](https://blog.rust-lang.org/2024/10/17/Rust-1.82.0.html).

Clone and build Nexum:

```sh
git clone https://github.com/nullisxyz/nexum
cd nexum
cargo install wasm-pack wasm-opt
wasm-pack build -t web --release -d ../../dist/pkg crates/worker
