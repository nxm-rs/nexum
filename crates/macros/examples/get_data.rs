#![allow(missing_docs)]
//! Example of using the apdu_pair macro to define a Get Data command

use nexum_apdu_core::{ApduCommand, StatusWord};
use nexum_apdu_macros::apdu_pair;

apdu_pair! {
    /// Get Data command (for ISO/IEC 7816-4 compliant cards)
    pub struct GetData {
        command {
            cla: 0x00,
            ins: 0xCA,
            secure: true,  // Often requires secure channel

            builders {
                /// Get data for a specific tag (P1-P2 are the tag)
                pub fn for_tag(tag: u16) -> Self {
                    Self::new((tag >> 8) as u8, (tag & 0xFF) as u8).with_le(0)
                }

                /// Get card serial number
                pub fn serial_number() -> Self {
                    Self::new(0x9F, 0x7F).with_le(0)
                }

                /// Get issuer data
                pub fn issuer_data() -> Self {
                    Self::new(0x42, 0x00).with_le(0)
                }
            }
        }

        response {
            variants {
                // Success
                #[sw(0x90, 0x00)]
                Success {
                    data: Vec<u8>,
                },

                // More data available
                #[sw(0x61, _)]
                MoreData {
                    sw2: u8,
                    data: Vec<u8>,
                },

                // Not found
                #[sw(0x6A, 0x88)]
                NotFound,

                // Security conditions not satisfied
                #[sw(0x69, 0x82)]
                SecurityNotSatisfied,

                // Other error
                #[sw(_, _)]
                OtherError {
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

                /// Get the status word
                pub fn status_word(&self) -> StatusWord {
                    match self {
                        Self::Success { .. } => StatusWord::new(0x90, 0x00),
                        Self::MoreData { sw2, .. } => StatusWord::new(0x61, *sw2),
                        Self::NotFound { .. } => StatusWord::new(0x6A, 0x88),
                        Self::SecurityNotSatisfied { .. } => StatusWord::new(0x69, 0x82),
                        Self::OtherError { sw1, sw2 } => StatusWord::new(*sw1, *sw2),
                    }
                }
            }
        }
    }
}

fn main() {
    // Example usage of the generated code:
    let cmd = GetDataCommand::serial_number();

    println!(
        "Get Data command: CLA={:#04x}, INS={:#04x}, P1={:#04x}, P2={:#04x}",
        cmd.class(),
        cmd.instruction(),
        cmd.p1(),
        cmd.p2()
    );

    // In a real application:
    // let response = executor.execute(&cmd).unwrap();
    // if let Some(data) = response.data() {
    //     println!("Serial Number: {:?}", data);
    // }
}
