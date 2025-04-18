# nexum-keycard: Rust Implementation for Keycards

`nexum-keycard` is a comprehensive toolkit for interacting with Keycards - secure smart cards designed for blockchain applications and cryptocurrency key management. This implementation provides a complete solution for Keycard operations in Rust.

[![docs.rs](https://img.shields.io/docsrs/nexum-keycard/latest)](https://docs.rs/nexum-keycard)
[![Crates.io](https://img.shields.io/crates/v/nexum-keycard)](https://crates.io/crates/nexum-keycard)

Build secure blockchain applications with hardware-backed security and the power of Rust.

## Installation

The easiest way to get started is to add the core crate:

```sh
cargo add nexum-keycard
```

For blockchain signing capabilities:

```sh
cargo add nexum-keycard-signer
```

For the command-line interface:

```sh
cargo install nexum-keycard-cli
```

## Quick Start

```rust
use nexum_keycard::{Keycard, PcscDeviceManager, CardExecutor, Error};

fn main() -> Result<(), Error> {
    // Create a PC/SC transport
    let manager = PcscDeviceManager::new()?;
    let readers = manager.list_readers()?;
    let reader = readers.iter().find(|r| r.has_card()).expect("No card present");
    let transport = manager.open_reader(reader.name())?;

    // Create a card executor
    let mut executor = CardExecutor::new_with_defaults(transport);

    // Create a Keycard instance and select the applet
    let mut keycard = Keycard::new(&mut executor);
    let app_info = keycard.select_keycard()?;

    println!("Selected Keycard with instance: {}", app_info.instance_uid);
    println!("Applet version: {}", app_info.version);

    // Initialize a new card (if needed)
    if !app_info.initialized() {
        let secrets = keycard.init(None, None, None)?;
        println!("Card initialized with:\nPIN: {}\nPUK: {}\nPairing password: {}",
                 secrets.pin(), secrets.puk(), secrets.pairing_password());
    }

    Ok(())
}
```

## Overview

This repository contains the following crates:

- [`nexum-keycard`]: Core functionality for interacting with Keycards
- [`nexum-keycard-signer`]: Alloy signer implementation for blockchain operations
- [`nexum-keycard-cli`]: Command-line interface for Keycard management

[`nexum-keycard`]: https://github.com/nullisxyz/keycard/tree/main/crates/keycard
[`nexum-keycard-signer`]: https://github.com/nullisxyz/keycard/tree/main/crates/signer
[`nexum-keycard-cli`]: https://github.com/nullisxyz/keycard/tree/main/crates/cli

## Features

- 🔐 **Secure Channel Communication** - Encrypted and authenticated channel to the card
- 🔑 **Key Management** - Generate, export, and manage keys on the Keycard
- 📝 **Credential Management** - Set and update PINs, PUKs, and pairing passwords
- 🔍 **Status Information** - Retrieve detailed info about the card status
- 🔄 **BIP32/39 Support** - Key derivation path support and mnemonic generation
- 📊 **Data Storage** - Store and retrieve custom data on the card
- 📱 **Factory Reset** - Complete card reset when needed
- 🌐 **Blockchain Integration** - Built-in support for Ethereum transaction signing

## Documentation & Examples

For detailed documentation on each crate, please check their individual `README` files:

- [`nexum-keycard` `README`](./crates/keycard/README.md) - Core Keycard functionality
- [`nexum-keycard-signer` `README`](./crates/signer/README.md) - Blockchain signer implementation
- [`nexum-keycard-cli` `README`](./crates/cli/README.md) - Command-line interface

## Command-Line Interface

nexum-keycard includes a comprehensive CLI for managing Keycards:

```sh
# List available readers
nexum-keycard-cli list

# Initialize a new card
nexum-keycard-cli init

# Generate a new key pair
nexum-keycard-cli generate-key

# Sign data
nexum-keycard-cli sign 0123456789abcdef --path m/44'/60'/0'/0/0
```

## Architecture

`nexum-keycard` is built on a layered architecture:

1. **APDU Transport Layer** - Handles low-level communication with card readers (via `nexum-apdu-*` crates)
2. **Secure Channel Layer** - Provides encryption and authentication for sensitive operations
3. **Keycard Command Layer** - Implements the Keycard protocol and commands
4. **Application Layer** - High-level APIs for key management and card operations

## License

Licensed under the [AGPL License](LICENSE) or http://www.gnu.org/licenses/agpl-3.0.html.

## Contributions

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in these crates by you shall be licensed as above, without any additional terms or conditions.
