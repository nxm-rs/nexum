#![allow(missing_docs)]
//! Example of using the apdu_pair macro to define a Read Record command

use nexum_apdu_core::{ApduCommand, StatusWord};
use nexum_apdu_macros::apdu_pair;

apdu_pair! {
    /// Read Record command
    pub struct ReadRecord {
        command {
            cla: 0x00,
            ins: 0xB2,
            secure: false,

            builders {
                /// Read a specific record by number from the current file
                pub fn record(record_number: u8, sfi: Option<u8>) -> Self {
                    let p2 = match sfi {
                        Some(sfi) => 0x04 | ((sfi & 0x1F) << 3), // Record in given SFI
                        None => 0x04,                            // Record in current file
                    };
                    Self::new(record_number, p2)
                }

                /// Read the first record from the current file
                pub fn first_record() -> Self {
                    Self::new(0x01, 0x04)
                }

                /// Read the next record from the current file
                pub fn next_record() -> Self {
                    Self::new(0x00, 0x02)
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

                // Record not found
                #[sw(0x6A, 0x83)]
                RecordNotFound,

                // End of records
                #[sw(0x6A, 0x82)]
                EndOfRecords,

                // Incorrect parameters
                #[sw(0x6A, _)]
                ParameterError {
                    sw2: u8,
                },

                // Other error
                #[sw(_, _)]
                OtherError {
                    sw1: u8,
                    sw2: u8,
                }
            }

            methods {
                /// Get the record data if available
                pub fn record_data(&self) -> Option<&[u8]> {
                    match self {
                        Self::Success { data } => Some(data),
                        _ => None,
                    }
                }

                /// Check if there are no more records
                pub fn is_end_of_records(&self) -> bool {
                    matches!(self, Self::EndOfRecords { .. } | Self::RecordNotFound { .. })
                }

                /// Get the status word
                pub fn status_word(&self) -> StatusWord {
                    match self {
                        Self::Success { .. } => StatusWord::new(0x90, 0x00),
                        Self::RecordNotFound { .. } => StatusWord::new(0x6A, 0x83),
                        Self::EndOfRecords { .. } => StatusWord::new(0x6A, 0x82),
                        Self::ParameterError { sw2 } => StatusWord::new(0x6A, *sw2),
                        Self::OtherError { sw1, sw2 } => StatusWord::new(*sw1, *sw2),
                    }
                }
            }
        }
    }
}

fn main() {
    // Example usage of the generated code:
    let cmd = ReadRecordCommand::first_record();

    println!(
        "Read Record command: CLA={:#04x}, INS={:#04x}, P1={:#04x}, P2={:#04x}",
        cmd.class(),
        cmd.instruction(),
        cmd.p1(),
        cmd.p2()
    );

    // In a real application:
    // let response = executor.execute(&cmd).unwrap();
    // if let Some(data) = response.record_data() {
    //     println!("Record: {:?}", data);
    // }
}
