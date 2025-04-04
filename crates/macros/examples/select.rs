//! Example of using the apdu_pair macro with the new Result-based API for Select command

use bytes::Bytes;
use nexum_apdu_core::ApduCommand;
use nexum_apdu_macros::apdu_pair;

apdu_pair! {
    /// Select command for applications and files
    pub struct Select {
        command {
            cla: 0x00,
            ins: 0xA4,

            builders {
                /// Select by name (AID)
                pub fn by_name(aid: impl Into<Bytes>) -> Self {
                    Self::new(0x04, 0x00).with_data(aid.into()).with_le(0)
                }

                /// Select by file ID
                pub fn by_file_id(file_id: impl Into<Bytes>) -> Self {
                    Self::new(0x00, 0x00).with_data(file_id.into()).with_le(0)
                }

                /// Select parent directory
                pub fn parent() -> Self {
                    Self::new(0x03, 0x00).with_le(0)
                }
            }
        }

        response {
            ok {
                // Normal success (90 00)
                #[sw(0x90, 0x00)]
                #[payload(field = "fci")]
                Selected {
                    fci: Vec<u8>,
                }
            }

            errors {
                // File or application not found (6A 82)
                #[sw(0x6A, 0x82)]
                #[error("File or application not found")]
                NotFound,

                // Incorrect parameters P1-P2 (6A 86)
                #[sw(0x6A, 0x86)]
                #[error("Incorrect parameters P1-P2")]
                IncorrectParameters,

                // Unknown error
                #[sw(_, _)]
                #[error("Unknown error: {sw1:02X}{sw2:02X}")]
                OtherError {
                    sw1: u8,
                    sw2: u8,
                }
            }

            methods {
                /// Returns true if selection was successful
                pub fn is_selected(&self) -> bool {
                    matches!(self, Self::Selected { .. })
                }

                /// Get the File Control Information if available
                pub fn fci(&self) -> Option<&Vec<u8>> {
                    match self {
                        Self::Selected { fci } => Some(fci),
                        _ => None,
                    }
                }
            }
        }
    }
}

fn main() {
    // Create a command to select a payment application
    let aid = [0xA0, 0x00, 0x00, 0x00, 0x03, 0x10, 0x10];
    let select_cmd = SelectCommand::by_name(aid.to_vec());

    println!(
        "Select command: CLA={:#04x}, INS={:#04x}, P1={:#04x}, P2={:#04x}",
        select_cmd.class(),
        select_cmd.instruction(),
        select_cmd.p1(),
        select_cmd.p2()
    );

    // Simulate a successful response
    let fci_data = [
        0x6F, 0x10, 0x84, 0x08, 0xA0, 0x00, 0x00, 0x00, 0x03, 0x10, 0x10, 0x00, 0xA5, 0x04, 0x9F,
        0x38, 0x01, 0x00,
    ];
    let response_bytes = Bytes::from([&fci_data[..], &[0x90, 0x00]].concat());

    let response = SelectResponse::from_bytes(&response_bytes).expect("Failed to parse response");

    // Old way
    if response.is_selected() {
        println!("Application selected successfully!");
        if let Some(fci) = response.fci() {
            println!("FCI: {:02X?}", fci);
        }
    }

    // New Result-based way
    match response.to_result() {
        Ok(ok) => match ok {
            SelectOk::Selected { fci } => {
                println!("Application selected successfully (via Result)!");
                println!("FCI data: {:02X?}", fci);
            }
        },
        Err(err) => {
            println!("Selection failed: {}", err);
        }
    }

    // Example function that uses the Result type alias
    fn select_application(_aid: &[u8]) -> SelectResult {
        // In a real application, this would use an executor
        // For example: executor.execute(&SelectCommand::by_name(aid))?.into()

        // Here we'll just simulate a response
        let fci_data = [
            0x6F, 0x10, 0x84, 0x08, 0xA0, 0x00, 0x00, 0x00, 0x03, 0x10, 0x10, 0x00, 0xA5, 0x04,
            0x9F, 0x38, 0x01, 0x00,
        ];
        let response_bytes = Bytes::from([&fci_data[..], &[0x90, 0x00]].concat());

        let response =
            SelectResponse::from_bytes(&response_bytes).map_err(|_| SelectError::OtherError {
                sw1: 0x6F,
                sw2: 0x00,
            })?;

        // Convert to Result - this allows us to use ? for error propagation
        response.into()
    }

    // Usage of our helper function
    match select_application(&aid) {
        Ok(ok) => match ok {
            SelectOk::Selected { fci } => {
                println!("Application selected via helper function!");
                println!("FCI length: {} bytes", fci.len());
            }
        },
        Err(err) => {
            println!("Selection via helper function failed: {}", err);
        }
    }
}
