#![allow(missing_docs)]
//! Example of using the apdu_pair macro with the new Result-based API

use nexum_apdu_core::prelude::*;
use nexum_apdu_macros::apdu_pair;

apdu_pair! {
    /// Get Data command (for ISO/IEC 7816-4 compliant cards)
    pub struct GetData {
        command {
            cla: 0x00,
            ins: 0xCA,

            builders {
                /// Get data for a specific tag (P1-P2 are the tag)
                pub fn for_tag(tag: u16) -> Self {
                    Self::new((tag >> 8) as u8, (tag & 0xFF) as u8).with_le(0)
                }

                /// Get card serial number
                pub fn serial_number() -> Self {
                    Self::new(0x9F, 0x7F).with_le(0)
                }
            }
        }

        response {
            ok {
                // Success with payload data
                #[sw(0x90, 0x00)]
                #[payload(field = "data")]
                Success {
                    data: Vec<u8>,
                },

                // More data available with partial payload
                #[sw(0x61, _)]
                #[payload(field = "data")]
                MoreData {
                    data: Vec<u8>,
                    sw2: u8,    // Automatically captures the "remaining bytes" from SW2
                },
            }

            errors {
                // Not found - no payload
                #[sw(0x6A, 0x88)]
                #[error("Referenced data not found")]
                NotFound,

                // Security conditions not satisfied
                #[sw(0x69, 0x82)]
                #[error("Security conditions not satisfied")]
                SecurityNotSatisfied,
            }
        }
    }
}

// Implement methods directly on the generated types
impl GetDataOk {
    /// Get the data
    pub fn data(&self) -> &[u8] {
        match self {
            Self::Success { data } => data,
            Self::MoreData { data, .. } => data,
        }
    }

    /// Get the number of remaining bytes for MoreData response
    pub fn remaining_bytes(&self) -> Option<u8> {
        match self {
            Self::MoreData { sw2, .. } => Some(*sw2),
            _ => None,
        }
    }
}

impl GetDataError {
    /// Check if the data was not found
    pub fn is_not_found(&self) -> bool {
        matches!(self, Self::NotFound)
    }

    /// Check if security conditions are not satisfied
    pub fn is_security_error(&self) -> bool {
        matches!(self, Self::SecurityNotSatisfied)
    }
}

fn main() {
    // Create a command
    let cmd = GetDataCommand::serial_number();

    println!(
        "Get Data command: CLA={:#04x}, INS={:#04x}, P1={:#04x}, P2={:#04x}",
        cmd.class(),
        cmd.instruction(),
        cmd.p1(),
        cmd.p2()
    );

    // Simulate a successful response
    let response_bytes = Bytes::from_static(&[0x01, 0x02, 0x03, 0x04, 0x90, 0x00]);
    let result = GetDataCommand::parse_response_raw(response_bytes);

    // Convert to inner result and use our custom methods
    match &result {
        Ok(ok) => {
            println!("Success! Data: {:?}", ok.data());
            if let Some(remaining) = ok.remaining_bytes() {
                println!("More data available: {} bytes", remaining);
            }
        }
        Err(err) => {
            println!("Error: {}", err);

            if err.is_not_found() {
                println!("Data not found on card");
            } else if err.is_security_error() {
                println!("Security condition not satisfied");
            }
        }
    }

    // Example of using ? operator with the result
    fn process_response(result: Result<GetDataOk, GetDataError>) -> Result<Vec<u8>, GetDataError> {
        // Get the inner result and use ?
        let ok = result?;

        // Use our custom method
        let data = ok.data().to_vec();

        // Check if there's more data
        if let Some(remaining) = ok.remaining_bytes() {
            println!("Note: {} more bytes available", remaining);
        }

        Ok(data)
    }

    // Use our function with the response
    match process_response(result) {
        Ok(data) => println!("Processed data: {:?}", data),
        Err(e) => println!("Processing error: {}", e),
    }
}
