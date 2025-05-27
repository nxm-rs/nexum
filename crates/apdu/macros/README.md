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
nexum-apdu-core = "0.1.0"
nexum-apdu-macros = "0.1.0"
```

### Basic Example

```rust
use nexum_apdu_core::prelude::*;
use nexum_apdu_macros::apdu_pair;

apdu_pair! {
    /// Select command
    pub struct Select {
        command {
            cla: 0x00,
            ins: 0xA4,
            required_security_level: SecurityLevel::none(),

            builders {
                /// Select by AID
                pub fn by_aid(aid: impl Into<Bytes>) -> Self {
                    Self::new(0x04, 0x00).with_data(aid.into()).with_le(0)
                }
            }
        }

        response {
            ok {
                #[sw(0x90, 0x00)]
                #[payload(field = "fci")]
                Success {
                    fci: Option<Vec<u8>>,
                },

                #[sw(0x61, _)]
                MoreData {
                    sw2: u8,
                }
            }

            errors {
                #[sw(0x6A, 0x82)]
                #[error("File not found")]
                NotFound,

                #[sw(_, _)]
                #[error("Other error: {sw1:02X}{sw2:02X}")]
                OtherError {
                    sw1: u8,
                    sw2: u8,
                }
            }

            custom_parse = |response: &Response| -> Result<SelectOk, SelectError> {
                // Optional custom parser implementation
                // This would override the default generated parser
                // ...
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
    // if response.is_ok() {
    //     println!("Application selected successfully!");
    // }
}
```

## Automatic Payload Handling

Mark which field should receive the response payload using the `#[payload]` attribute:

```rust
ok {
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
            ok {
                #[sw(0x90, 0x00)]
                Success {
                    parsed_data: Vec<u8>,
                },
                // Other variants...
            }

            errors {
                // Error variants...
            }

            custom_parse = |response: &Response| -> Result<GetDataOk, GetDataError> {
                let status = response.status();
                let sw1 = status.sw1;
                let sw2 = status.sw2;
                let data_payload = response.payload();

                match (sw1, sw2) {
                    (0x90, 0x00) => {
                        // Custom parsing logic here
                        let mut parsed_data = Vec::new();
                        if let Some(payload) = data_payload {
                            if !payload.is_empty() {
                                if payload[0] != expected_tag {
                                    return Err(GetDataError::ResponseError(
                                        nexum_apdu_core::response::error::ResponseError::Message(
                                            "Invalid tag".to_string()
                                        )
                                    ));
                                }
                                parsed_data.extend_from_slice(&payload[1..]);
                            }
                        }
                        Ok(GetDataOk::Success { parsed_data })
                    },
                    // Handle other status word combinations...
                    (sw1, sw2) => Err(GetDataError::Unknown { sw1, sw2 }),
                }
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

## Generated Types

The macro generates the following types:

- `{Name}Command` - The APDU command struct
- `{Name}Result` - A Result-like wrapper for the response
- `{Name}Ok` - Enum of success response variants
- `{Name}Error` - Enum of error response variants

## Integration with Prelude

The generated command and response types work seamlessly with the `nexum_apdu_core::prelude` module:

```rust
use nexum_apdu_core::prelude::*;
use nexum_apdu_macros::apdu_pair;
use nexum_apdu_transport_pcsc::PcscDeviceManager;

// Define command/response pair
apdu_pair! { /* ... */ }

fn main() -> Result<(), Error> {
    // Set up transport and executor
    let manager = PcscDeviceManager::new()?;
    // ...
    let mut executor = CardExecutor::new_with_defaults(transport);

    // Use generated command
    let select_cmd = SelectCommand::by_aid([0xA0, 0x00, 0x00, 0x00, 0x03, 0x10, 0x10]);

    // Execute and handle response
    let result = executor.execute(&select_cmd)?;
    match &*result {
        SelectOk::Success { fci } => {
            println!("Selected successfully, FCI: {:?}", fci);
        },
        SelectOk::MoreData { sw2 } => {
            println!("More data available: {} bytes", sw2);
        }
    }

    Ok(())
}
```

## License

Licensed under the [AGPL License](../../LICENSE) or http://www.gnu.org/licenses/agpl-3.0.html.

## Contributions

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in these crates by you shall be licensed as above, without any additional terms or conditions.
