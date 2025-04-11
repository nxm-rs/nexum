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
- Streamlined imports through the prelude module

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
        command: &Command,
        transport: &mut dyn CardTransport
    ) -> Result<Response, ProcessorError>;

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

## Using the Prelude

To simplify imports, you can use the prelude module:

```rust
use nexum_apdu_core::prelude::*;
```

This provides access to all commonly used types and traits:

- Core types: `Bytes`, `BytesMut`, `Error`
- Command types: `Command`, `ApduCommand`, `CommandResult`, `ExpectedLength`
- Response types: `Response`, `ApduResponse`, `StatusWord`
- Transport layer: `CardTransport`, `TransportError`
- Processor layer: `CommandProcessor`, `ProcessorError`, common processors
- Executor layer: `Executor`, `CardExecutor`, extension traits

## Example

```rust
use nexum_apdu_core::prelude::*;
use some_transport::PcscTransport;

fn main() -> Result<(), Error> {
    // Create a transport
    let transport = PcscTransport::connect("Smartcard Reader 0")?;

    // Create an executor with default processors (GET RESPONSE handler)
    let mut executor = CardExecutor::new_with_defaults(transport);

    // Create a SELECT command to select a payment application
    let aid = [0xA0, 0x00, 0x00, 0x00, 0x03, 0x10, 0x10];
    let select_cmd = Command::new_with_data(0x00, 0xA4, 0x04, 0x00, aid.to_vec());

    // Execute the command
    let response = executor.transmit(&select_cmd)?;

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
executor.open_secure_channel(&provider)?;

// Commands now automatically use the secure channel
let response = executor.transmit(&command)?;
```

## Command Processors

The library includes various built-in command processors:

- `GetResponseProcessor`: Automatically handles GET RESPONSE commands for response chaining
- `IdentityProcessor`: Simple pass-through processor for testing

You can implement custom processors to handle:
- Secure messaging
- Command pre-processing
- Response post-processing
- Protocol-specific translations

## Feature Flags

- `std` (default): Enable standard library support
- `longer_payloads`: Enable support for extended length APDUs

## License

Licensed under the [AGPL License](../../LICENSE) or http://www.gnu.org/licenses/agpl-3.0.html.

## Contributions

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in these crates by you shall be licensed as above, without any additional terms or conditions.
