# APDU Core

Core traits and types for smart card APDU (Application Protocol Data Unit) operations.

This crate provides the foundational types and traits needed for working with smart card commands and responses according to ISO/IEC 7816-4.

## Features

- Generic and flexible APDU command/response abstractions
- Transport layer for different card communication methods
- Command processor pipeline for flexible transformations
- Support for secure channels
- Comprehensive error handling and status word interpretation
- Detailed tracing for debugging
- No-std compatible for embedded environments

## Architecture

The crate is built around three main abstractions:

### Transport Layer

The `CardTransport` trait represents the low-level communication with a card:

```rust
pub trait CardTransport: Send + Sync + fmt::Debug {
    fn transmit_raw(&mut self, command: &[u8]) -> Result<Bytes, TransportError>;
    fn is_connected(&self) -> bool;
    fn reset(&mut self) -> Result<(), TransportError>;
}
```

### Command Processor Layer

The `CommandProcessor` trait handles command transformations:

```rust
pub trait CommandProcessor: Send + Sync + fmt::Debug + DynClone {
    fn process_command(
        &mut self,
        command: &[u8],
        transport: &mut dyn CardTransport,
    ) -> Result<Bytes, ProcessorError>;

    fn security_level(&self) -> SecurityLevel;
    fn is_active(&self) -> bool;
}
```

### Executor Layer

The `Executor` trait manages the complete command execution flow:

```rust
pub trait Executor: Send + Sync + fmt::Debug {
    fn transmit(&mut self, command: &[u8]) -> Result<Bytes>;
    fn security_level(&self) -> SecurityLevel;
    fn reset(&mut self) -> Result<()>;
}
```

## Example

```rust
use apdu_core::{Command, CardExecutor, Response};
use apdu_core::processor::GetResponseProcessor;
use some_transport::PcscTransport;

fn main() -> Result<(), apdu_core::Error> {
    // Create a transport
    let transport = PcscTransport::connect("Smartcard Reader 0")?;

    // Create an executor with default processors (GET RESPONSE handler)
    let mut executor = CardExecutor::new_with_defaults(transport);

    // Create a SELECT command to select a payment application
    let aid = [0xA0, 0x00, 0x00, 0x00, 0x03, 0x10, 0x10];
    let select_cmd = Command::new_with_data(0x00, 0xA4, 0x04, 0x00, aid.to_vec());

    // Execute the command
    let response = executor.execute(&select_cmd)?;

    // Parse the response
    if response.is_success() {
        println!("Application selected successfully");
    } else {
        println!("Failed to select application: {}", response.status());
    }

    Ok(())
}
```

## Secure Channels

Secure channels are implemented as command processors:

```rust
// Create a secure channel provider
let provider = SomeSecureChannelProvider::new(keys);

// Open a secure channel with the card
executor.open_secure_channel(&provider, SecurityLevel::FullEncryption)?;

// Commands now automatically use the secure channel
let response = executor.transmit(&command)?;
```

## No-std Support

This crate supports `no_std` environments. To use it without the standard library, disable the default features:

```toml
[dependencies]
apdu-core = { version = "0.1.0", default-features = false }
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
