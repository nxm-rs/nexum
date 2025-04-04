#![allow(missing_docs)]
//! Example of using the apdu_pair macro with the new Result-based API

use bytes::Bytes;
use nexum_apdu_core::ApduCommand;
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

                // Other error
                #[sw(_, _)]
                #[error("Unknown error: {sw1:02X}{sw2:02X}")]
                Other {
                    sw1: u8,
                    sw2: u8,
                }
            }

            methods {
                /// Get the data if available
                pub fn data(&self) -> Option<&[u8]> {
                    match self {
                        Self::Success { data } => Some(data),
                        Self::MoreData { data, .. } => Some(data),
                        _ => None,
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
        }
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
    let response = GetDataResponse::from_bytes(&response_bytes).expect("Failed to parse response");

    // Using the traditional approach
    if let GetDataResponse::Success { data } = &response {
        println!("Success! Data: {:?}", data);
    }

    // Convert to Result (the new way)
    match response.clone().to_result() {
        Ok(success) => match success {
            GetDataOk::Success { data } => {
                println!("Success via Result! Data: {:?}", data);
            }
            GetDataOk::MoreData { data, sw2 } => {
                println!("More data available: {:?}, remaining: {}", data, sw2);
            }
        },
        Err(err) => {
            println!("Error: {}", err);
        }
    }

    // Shorter version using ? operator
    fn process_response(response: GetDataResponse) -> Result<Vec<u8>, GetDataError> {
        // Using the From implementation to convert to Result
        let result: GetDataResult = response.into();

        // Now we can use ? operator
        let success = result?;

        // Match on success variants
        match success {
            GetDataOk::Success { data } => Ok(data),
            GetDataOk::MoreData { data, sw2 } => {
                println!("Note: {} more bytes available", sw2);
                Ok(data)
            }
        }
    }

    // Use our function with the response
    match process_response(response) {
        Ok(data) => println!("Processed data: {:?}", data),
        Err(e) => println!("Processing error: {}", e),
    }
}
