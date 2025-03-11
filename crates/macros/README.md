# APDU Macros

Procedural macros for APDU (Application Protocol Data Unit) operations used in smart card communications.

This crate provides macros to simplify the definition of APDU commands and responses according to ISO/IEC 7816-4 standards.

## Features

- Define APDU commands and their associated responses together
- Status word-based response variant handling
- Builder methods for common command patterns
- Support for capturing status word values in response types
- TLV data field support (coming soon)

## Usage

Add the dependencies to your `Cargo.toml`:

```toml
[dependencies]
apdu-core = "0.1.0"
apdu-macros = "0.1.0"
```

### Basic Example

```rust
use apdu_core::{ApduCommand, Bytes, CommandExecutor};
use apdu_macros::apdu_pair;

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
            enum_response {
                Success {
                    #[sw(0x90, 0x00)]
                    fci: Option<Vec<u8>>,
                },

                NotFound {
                    #[sw(0x6A, 0x82)]
                },

                OtherError {
                    #[sw(_, _)]
                    #[sw1]
                    sw1: u8,
                    #[sw2]
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

## Status Word Matching

The macros provide several ways to match status words:

```rust
// Match exact SW
#[sw(0x90, 0x00)]

// Match any SW2 when SW1 is 0x61
#[sw(0x61, _)]

// Match any SW1 and SW2
#[sw(_, _)]

// Capture SW1 and SW2 values
#[sw(_, _)]
#[sw1]
sw1: u8,
#[sw2]
sw2: u8,
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you shall be dual licensed as above, without any additional terms or conditions.
