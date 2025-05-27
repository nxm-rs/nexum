# nexum-keycard-signer: Ethereum Signer Implementation for Keycard

`nexum-keycard-signer` provides an implementation of the [`alloy-signer`](https://crates.io/crates/alloy-signer) trait for Keycards, allowing them to be used as hardware signers for Ethereum and other EVM-compatible blockchains.

[![docs.rs](https://img.shields.io/docsrs/nexum-keycard-signer/latest)](https://docs.rs/nexum-keycard-signer)
[![Crates.io](https://img.shields.io/crates/v/nexum-keycard-signer)](https://crates.io/crates/nexum-keycard-signer)

Secure your blockchain transactions with hardware-backed signing using Keycards and the Alloy ecosystem.

## Installation

```sh
cargo add nexum-keycard-signer
```

You'll also need the core keycard crate:

```sh
cargo add nexum-keycard
```

## Quick Start

```rust
use alloy_signer::Signer;
use nexum_keycard::{Keycard, PcscDeviceManager, CardExecutor};
use nexum_keycard_signer::{KeycardSigner, DerivationPath};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // Set up the Keycard
    let manager = PcscDeviceManager::new()?;
    let readers = manager.list_readers()?;
    let reader = readers.iter().find(|r| r.has_card()).expect("No card present");
    let transport = manager.open_reader(reader.name())?;
    let mut executor = CardExecutor::new_with_defaults(transport);

    // Create a Keycard instance
    let mut keycard = Keycard::new(&mut executor);
    keycard.select()?;

    // Pair and authenticate (assuming the card is already initialized)
    let pairing_info = keycard.pair("YOUR_PAIRING_PASSWORD")?;
    keycard.open_secure_channel(pairing_info)?;
    keycard.verify_pin("YOUR_PIN")?;

    // Create a signer with a specific derivation path
    let path = DerivationPath::from_str("m/44'/60'/0'/0/0")?;
    let signer = KeycardSigner::new(keycard, path);

    // Get the Ethereum address
    let address = signer.address();
    println!("Ethereum address: {}", address);

    // Sign a message (example)
    let message = [1u8, 2, 3, 4, 5];
    let signature = signer.sign_message(&message).await?;
    println!("Signature: {:?}", signature);

    Ok(())
}
```

For more complete examples, check out the examples directory in the repository.

## Features

- 🔐 **Hardware-backed Security** - Private keys never leave the secure element
- 🔄 **BIP32 Support** - Full derivation path support for hierarchical wallets
- 📱 **Alloy Integration** - Seamless integration with the Alloy ecosystem
- 🔌 **Asynchronous API** - Non-blocking signing operations
- 🌐 **Multi-chain Support** - Works with any EVM-compatible blockchain

## Implementation Details

This crate implements the `Signer` trait from `alloy-signer`, providing the following functionality:

- Address derivation from public key
- Message signing (both raw and Ethereum-formatted messages)
- Transaction signing compliant with EIP-155

The implementation delegates all cryptographic operations to the secure element on the Keycard, ensuring that private keys never leave the hardware.

## Related Crates

- [`nexum-keycard`](https://crates.io/crates/nexum-keycard) - Core functionality for interacting with Keycards
- [`nexum-keycard-cli`](https://crates.io/crates/nexum-keycard-cli) - Command-line interface for Keycard management

## License

Licensed under the [AGPL License](../../LICENSE) or http://www.gnu.org/licenses/agpl-3.0.html.

## Contributions

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this crate by you shall be licensed as above, without any additional terms or conditions.
