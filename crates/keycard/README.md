# nexum-keycard: Keycard Implementation in Rust

`nexum-keycard` is a Rust implementation for interacting with Keycards - secure hardware wallets in a smart card form factor. It provides a comprehensive API for managing Keycards, including secure channel communication, key management, and cryptographic operations.

[![docs.rs](https://img.shields.io/docsrs/nexum-keycard/latest)](https://docs.rs/nexum-keycard)
[![Crates.io](https://img.shields.io/crates/v/nexum-keycard)](https://crates.io/crates/nexum-keycard)

Secure your blockchain private keys with hardware security in a convenient card format, powered by robust Rust implementation.

## Installation

```sh
cargo add nexum-keycard
```

For PC/SC reader support (usually needed):

```sh
cargo add nexum-apdu-transport-pcsc
```

## Features

- 🔐 **Secure Channel Communication** - Encrypted and authenticated channel to the card
- 🔑 **Key Management** - Generate, export, and manage keys on the Keycard
- 📝 **Credential Management** - Set and update PINs, PUKs, and pairing passwords
- 🔍 **Status Information** - Retrieve detailed info about the card status
- 🔄 **BIP32/39 Support** - Key derivation path support and mnemonic generation
- 📊 **Data Storage** - Store and retrieve custom data on the card
- 📱 **Factory Reset** - Complete card reset when needed

## Usage Examples

### Initializing a New Card

```rust
// Initialize with random or specific credentials
let pin = "123456"; // Default is a random 6-digit number
let puk = "123456789012"; // Default is a random 12-digit number
let pairing_password = "KeycardTest"; // Default is random

let secrets = keycard.init(Some(pin), Some(puk), Some(pairing_password))?;
println!("Card initialized with:");
println!("PIN: {}", secrets.pin());
println!("PUK: {}", secrets.puk());
println!("Pairing password: {}", secrets.pairing_password());
```

### Pairing with a Card

```rust
let pairing_info = keycard.pair("KeycardTest")?;
println!("Paired successfully. Pairing index: {}", pairing_info.index());
```

### Generating Keys

```rust
// Open a secure channel interactively or programmatically
keycard.open_secure_channel()?;

// Authenticate with PIN
keycard.verify_pin("123456")?;

// Generate a new key pair
keycard.generate_key()?;
println!("Generated new key pair successfully");
```

## Implementation Details

`nexum-keycard` implements the full Keycard protocol specification, allowing for seamless interaction with hardware Keycards. It utilizes the `nexum-apdu` framework for smart card communication.

The library is designed with a layered approach:

1. **Transport Layer** - Uses PC/SC to communicate with physical card readers
2. **Secure Channel Layer** - Implements encryption and authentication
3. **Command Layer** - Provides high-level API for Keycard operations

## Related Crates

- [`nexum-keycard-signer`](https://crates.io/crates/nexum-keycard-signer) - Alloy signer implementation using Keycard
- [`nexum-keycard-cli`](https://crates.io/crates/nexum-keycard-cli) - Command-line interface for Keycard management

## License

Licensed under the [AGPL License](../../LICENSE) or http://www.gnu.org/licenses/agpl-3.0.html.

## Contributions

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this crate by you shall be licensed as above, without any additional terms or conditions.
