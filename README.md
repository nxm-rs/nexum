# Nexum

<!--[![CI status](https://github.com/your-org/nexum/workflows/CI/badge.svg)][gh-ci]-->
<!--[![Telegram Chat][tg-badge]][tg-url]-->

**Blazing-fast Ethereum access, where Rust brings web and terminal together for hackers.**

<!-- ![](./assets/nexum-banner.png) -->

<!--**[Install](https://your-org.github.io/nexum/installation)**
| [User Guide](https://nexum.rs)
| [Developer Docs](./docs)
| [Crate Docs](https://docs.rs/nexum)-->

<!--[gh-ci]: https://github.com/your-org/nexum/actions/workflows/ci.yml -->
<!-- [tg-badge]: https://img.shields.io/endpoint?color=neon&logo=telegram&label=chat&url=https%3A%2F%2Ftg.sumanjay.workers.dev%2Fnexum -->

## What is Nexum?

Nexum is a high-performance Ethereum provider written in Rust and compiled to WebAssembly, designed for both web extension environments and terminal interfaces. Originally forked from [frame-extension](https://github.com/frame-labs/frame-extension), Nexum is a fully compliant `EIP-1193` provider that brings together secure, fast, and hacker-friendly access to Ethereum.

With Nexum, users can connect to the Ethereum network through a unique setup that combines **WebTransport** with a **terminal-based** user interface wallet. This makes Nexum ideal for developers and hackers seeking a flexible, performant tool for interacting with Ethereum on the web and beyond.

## Goals

Nexum is built with the following goals:

1. **Compliance**: Nexum aims to be fully [`EIP-1193`](https://eips.ethereum.org/EIPS/eip-1193) and [`EIP-6963`](https://eips.ethereum.org/EIPS/eip-6963) compliant, ensuring seamless integration with Ethereum dApps and tools that rely on this standard provider API.
2. **Performance**: Built in Rust and compiled to WASM, Nexum ensures blazing-fast performance for both web and terminal access, with secure memory management and optimized response times.
3. **Web and Terminal Integration**: Nexum leverages [WebTransport](https://developer.mozilla.org/en-US/docs/Web/API/WebTransport) to provide a consistent and secure connection between the service worker and terminal interface, creating a powerful and seamless experience for web and terminal users alike.
4. **Hackable**: Nexum is designed for hackers, with a developer-first approach to Ethereum access. It supports flexible use in decentralized applications, testing environments, and experimental setups.

## Status

Nexum is currently in active development and intended for those looking to explore fast, `EIP-1193` compliant Ethereum access in a dual web/terminal environment. We invite developers and enthusiasts to contribute, test, and provide feedback as we expand Nexum's capabilities.

## For Users

Guide coming soon!
<!-- See the [Nexum Guide](https://nexum.rs) for installation instructions and usage examples. -->

## For Developers

### Using Nexum as a Library

<!-- Nexum’s components can be used as standalone crates for Rust projects. See the [Crate Docs](https://docs.rs/nexum) for detailed API documentation. -->

Crate docs coming soon!

### Contributing

Nexum is an open-source project and welcomes contributions from the community! If you'd like to help improve Nexum, you can:

<!-- - Review our contributor guidelines in [`CONTRIBUTING.md`](./CONTRIBUTING.md). -->
- Join the [Signal](https://signal.group/#CjQKIHNV-kWphhtnpwS3zywC7LRr5BEW9Q1XyDl2qZtL2WYqEhAyO0c8tGmrQDmEsY15rALt) group to discuss Nexum's development.
- Open an [issue](https://github.com/nullisxyz/nexum/issues) with your ideas or questions.

### Building and Testing

The Minimum Supported Rust Version (MSRV) of Nexum is [1.82.0](https://blog.rust-lang.org/2024/10/17/Rust-1.82.0.html).

To clone Nexum locally:

```sh
git clone https://github.com/nullisxyz/nexum
cd nexum
```
<!-- add test instructions above -->

<!-- To speed up testing, we recommend using [`cargo nextest`](https://nexte.st/). With nextest installed, simply substitute `cargo test` with `cargo nextest run`. -->

## Acknowledgements

Nexum's web extension was originally forked from [`frame-extension`](https://github.com/your-org/frame-extension) and draws inspiration from its modular design. Big thanks to the [Frame.sh](https://frame.sh) community for laying the groundwork!

<!-- ## Security -->

<!-- See [`SECURITY.md`](./SECURITY.md) for guidelines on reporting security issues. -->

<!-- ## Getting Help

- Visit the [User Guide](https://nexum.rs) for detailed instructions.
- Join the [Telegram Chat][tg-url] for community support.
- Open an issue or discussion on GitHub for additional help. -->

## License

Nexum is licensed under the GPL-3.0 license. See [`LICENSE`](./LICENSE) for more information.
