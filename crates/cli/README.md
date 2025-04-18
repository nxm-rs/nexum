# nexum-keycard-cli: Command-Line Interface for Keycard Management

`nexum-keycard-cli` provides a comprehensive command-line interface for managing Keycards, allowing users to initialize cards, generate keys, sign transactions, and perform various administrative operations.

[![Crates.io](https://img.shields.io/crates/v/nexum-keycard-cli)](https://crates.io/crates/nexum-keycard-cli)

Manage your Keycard directly from the terminal with this powerful, feature-rich CLI tool.

## Installation

### From crates.io (soon)

```sh
cargo install nexum-keycard-cli
```

### From source

```sh
git clone https://github.com/nullisxyz/keycard
cd keycard
cargo install --path crates/cli
```

## Usage

```
nexum-keycard-cli [OPTIONS] <COMMAND>
```

### Options

- `-r, --reader <READER>` - Specify which card reader to use (otherwise autodetects)
- `-v, --verbose` - Enable verbose logging for debugging
- `-h, --help` - Print help
- `-V, --version` - Print version

### Commands

- `list` - List available card readers
- `select` - Select the Keycard applet and show card information
- `init` - Initialize a new card with PIN, PUK and pairing password
- `pair` - Pair with a Keycard and save pairing information
- `generate-key` - Generate a new key pair on the card
- `export-key` - Export a public key (or private key) from the card
- `sign` - Sign data with the key on the card
- `change-credential` - Change PIN, PUK or pairing password
- `unblock-pin` - Unblock PIN using PUK
- `set-pinless-path` - Set a path for PIN-less signing
- `load-key-from-seed` - Load a key from BIP39 seed phrase
- `remove-key` - Remove the current key from the card
- `get-status` - Show detailed status information about the card
- `unpair` - Remove pairing information from the card
- `generate-mnemonic` - Generate a BIP39 mnemonic phrase on the card
- `store-data` - Store arbitrary data on the card
- `get-data` - Retrieve stored data from the card
- `factory-reset` - Reset the card to factory settings
- `applet` - Applet management commands

## Examples

### Initialize a new card

```sh
nexum-keycard-cli init
```

To specify custom values:

```sh
nexum-keycard-cli init --pin 123456 --puk 123456789012 --pairing-password MyPassword
```

### Pair with a card

```sh
nexum-keycard-cli pair --output pairing.json
```

### Generate a key

```sh
nexum-keycard-cli generate-key --file pairing.json
```

### Sign data

```sh
nexum-keycard-cli sign 0123456789abcdef --path m/44'/60'/0'/0/0 --file pairing.json
```

### Get card status

```sh
nexum-keycard-cli get-status --file pairing.json
```

## Features

- 🔧 **Complete Card Management** - Initialize, pair, reset, and manage card credentials
- 🔐 **Key Operations** - Generate, export, and use keys for cryptographic operations
- 💳 **PIN Management** - Set, change, and unblock PINs and PUKs
- 🔑 **Secure Pairing** - Establish encrypted connections with your card
- 📝 **Data Storage** - Store and retrieve arbitrary data
- 🧩 **BIP32/39 Support** - Create and use hierarchical deterministic wallets
- 📋 **Factory Reset** - Complete card reset when needed

## Pairing and Security

The CLI supports secure channel communication with Keycards to protect sensitive operations. Pairing information can be saved to a file for reuse across multiple commands.

For security-critical operations, you'll need to provide your PIN (and sometimes PUK) to authenticate.

## Related Crates

- [`nexum-keycard`](https://crates.io/crates/nexum-keycard) - Core functionality for interacting with Keycards
- [`nexum-keycard-signer`](https://crates.io/crates/nexum-keycard-signer) - Alloy signer implementation for Keycards

## License

Licensed under the [AGPL License](../../LICENSE) or http://www.gnu.org/licenses/agpl-3.0.html.

## Contributions

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this crate by you shall be licensed as above, without any additional terms or conditions.
