# APDU Macros

Procedural macros for APDU (Application Protocol Data Unit) operations used in smart card communications.

This crate provides macros to simplify the definition of APDU commands and responses according to ISO/IEC 7816-4 standards.

## Features

- Define APDU commands and their associated responses together
- Status word-based response variant handling
- Builder methods for common command patterns
- Support for capturing status word values in response types
- Custom payload parsing support
- Flexible status word matching patterns

## Usage

Add the dependencies to your `Cargo.toml`:

```toml
[dependencies]
apdu-core = "0.1.0"
apdu-macros = "0.1.0"
```

### Basic Example

```rust
use nexum_apdu_core::{ApduCommand, Bytes, CommandExecutor};
use nexum_apdu_macros::apdu_pair;

apdu_pair! {
    /// Select command
    pub struct Select {
        command {
            cla: 0x00,
            ins: 0xA4,
            secure: false,

            builders {
                /// Select by AID
                pub fn by_aid(aid: impl Into<Bytes>) -> Self {
                    Self::new(0x04, 0x00).with_data(aid.into()).with_le(0)
                }
            }
        }

        response {
            variants {
                #[sw(0x90, 0x00)]
                Success {
                    fci: Option<Vec<u8>>,
                },

                #[sw(0x6A, 0x82)]
                NotFound,

                #[sw(_, _)]
                #[sw1]
                OtherError {
                    sw1: u8,
                    sw2: u8,
                }
            }

            methods {
                pub fn is_success(&self) -> bool {
                    matches!(self, Self::Success { .. })
                }
            }
        }
    }
}

// Usage example
fn main() {
    let aid = [0xA0, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00];
    let select_cmd = SelectCommand::by_aid(aid);

    // Now use the command with an executor
    // let response = executor.execute(&select_cmd).unwrap();
    // if response.is_success() {
    //     println!("Application selected successfully!");
    // }
}
```

## Custom Payload Parsing

You can provide custom payload parsing logic:

```rust
apdu_pair! {
    pub struct GetData {
        // Command section...

        response {
            variants {
                #[sw(0x90, 0x00)]
                Success {
                    parsed_data: Vec<u8>,
                },
                // Other variants...
            }

            parse_payload = |payload, sw, variant| -> Result<(), nexum_apdu_core::Error> {
                if let Self::Success { parsed_data } = variant {
                    // Custom parsing logic here
                    parsed_data.extend_from_slice(payload);
                }
                Ok(())
            }

            methods {
                // Custom methods...
            }
        }
    }
}
```

## Status Word Matching

The macros provide several ways to match status words:

```rust
// Match exact SW
#[sw(0x90, 0x00)]

// Match any SW2 when SW1 is 0x61
#[sw(0x61, _)]

// Match any SW1 and SW2
#[sw(_, _)]

// Match using a StatusWord constant
#[sw(status::SUCCESS)]

// Capture SW1 and SW2 values in fields
#[sw(_, _)]
OtherError {
    sw1: u8,
    sw2: u8,
}
```

## License

Licensed under MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT).

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you shall be licensed as above, without any additional terms or conditions.
