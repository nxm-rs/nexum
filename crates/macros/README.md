# APDU Macros

Procedural macros for APDU (Application Protocol Data Unit) operations used in smart card communications.

This crate provides macros to simplify the definition of APDU commands and responses according to ISO/IEC 7816-4 standards.

## Features

- Define APDU commands and their associated responses together
- Status word-based response variant handling
- Builder methods for common command patterns
- Support for capturing status word values in response types
- Automatic or custom payload parsing
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
                #[payload(field = "fci")]
                Success {
                    fci: Option<Vec<u8>>,
                },

                #[sw(0x6A, 0x82)]
                NotFound,

                #[sw(_, _)]
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

## Automatic Payload Handling

Mark which field should receive the response payload using the `#[payload]` attribute:

```rust
variants {
    #[sw(0x90, 0x00)]
    #[payload(field = "data")]
    Success {
        data: Vec<u8>,  // Will automatically receive the response payload
    },

    #[sw(0x61, _)]
    #[payload(field = "partial_data")]
    MoreData {
        partial_data: Vec<u8>,  // Will receive the partial payload
        sw2: u8,  // Automatically captures SW2 value
    },
}
```

The macro intelligently handles different payload types:
- `Vec<u8>` and `bytes::Bytes` - Direct assignment
- `Option<Vec<u8>>` - Wraps payload in Some() if present
- `String` - Attempts UTF-8 conversion

## Custom Payload Parsing

For more complex cases, use `custom_parse` to gain full control over response parsing:

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

            custom_parse = |payload, sw| -> Result<GetDataResponse, nexum_apdu_core::Error> {
                match (sw.sw1(), sw.sw2()) {
                    (0x90, 0x00) => {
                        // Custom parsing logic here
                        let mut parsed_data = Vec::new();
                        if !payload.is_empty() {
                            // Validate and transform the payload
                            if payload[0] != expected_tag {
                                return Err(nexum_apdu_core::Error::Parse("Invalid tag"));
                            }
                            parsed_data.extend_from_slice(&payload[1..]);
                        }
                        Ok(GetDataResponse::Success { parsed_data })
                    },
                    // Handle other status word combinations...
                    (sw1, sw2) => Ok(GetDataResponse::OtherError { sw1, sw2 }),
                }
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
