#![allow(missing_docs)]
//! Example of using the apdu_pair macro with the new Result-based API for Read Record command

use bytes::Bytes;
use nexum_apdu_core::{ApduCommand, ApduResponse};
use nexum_apdu_macros::apdu_pair;

apdu_pair! {
    /// Read Record command
    pub struct ReadRecord {
        command {
            cla: 0x00,
            ins: 0xB2,

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
            ok {
                // Success
                #[sw(0x90, 0x00)]
                #[payload(field = "data")]
                Success {
                    data: Vec<u8>,
                }
            }

            errors {
                // Record not found
                #[sw(0x6A, 0x83)]
                #[error("Record not found")]
                RecordNotFound,

                // End of records
                #[sw(0x6A, 0x82)]
                #[error("End of records")]
                EndOfRecords,

                // Incorrect parameters
                #[sw(0x6A, _)]
                #[error("Parameter error: SW2={sw2:02X}")]
                ParameterError {
                    sw2: u8,
                },
            }
        }
    }
}

// Implement methods directly on the generated types
impl ReadRecordOk {
    /// Get the record data
    pub fn record_data(&self) -> &[u8] {
        match self {
            Self::Success { data } => data,
        }
    }
}

impl ReadRecordError {
    /// Check if there are no more records
    pub fn is_end_of_records(&self) -> bool {
        matches!(self, Self::EndOfRecords | Self::RecordNotFound)
    }

    /// Check if this is a parameter error
    pub fn is_parameter_error(&self) -> bool {
        matches!(self, Self::ParameterError { .. })
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

    // Simulate a successful response
    let record_data = [
        0x70, 0x12, 0x5A, 0x08, 0x41, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x5F, 0x24, 0x03,
        0x23, 0x12, 0x31, 0x9F, 0x1F, 0x01, 0x00,
    ];
    let response_bytes = Bytes::from([&record_data[..], &[0x90, 0x00]].concat());

    let result = ReadRecordResult::from_bytes(&response_bytes).unwrap();

    // Using our custom methods on the unwrapped result
    match result.into_inner() {
        Ok(ok) => {
            println!("Record data: {:02X?}", ok.record_data());
        }
        Err(err) => {
            println!("Error reading record: {}", err);

            if err.is_end_of_records() {
                println!("No more records available");
            } else if err.is_parameter_error() {
                println!("Invalid parameters used in command");
            }
        }
    }

    // Function that demonstrates error handling with Result
    fn read_all_records(_sfi: u8) -> Result<Vec<Vec<u8>>, ReadRecordError> {
        let mut records = Vec::new();
        let mut record_number = 1;

        loop {
            // In a real application, this would use an executor
            // For this example, we'll simulate responses

            let result = if record_number <= 3 {
                // Simulate a success response for first 3 records
                let record_data = [
                    0x70,
                    0x12,
                    0x5A,
                    0x08,
                    0x41,
                    0x11,
                    0x22,
                    0x33,
                    0x44,
                    0x55,
                    0x66,
                    0x77,
                    0x5F,
                    0x24,
                    0x03,
                    0x23,
                    0x12,
                    0x31,
                    0x9F,
                    0x1F,
                    0x01,
                    record_number,
                ];
                let response_bytes = Bytes::from([&record_data[..], &[0x90, 0x00]].concat());

                ReadRecordResult::from_bytes(&response_bytes)
            } else {
                // Simulate end of records for record 4+
                let response_bytes = Bytes::from_static(&[0x6A, 0x83]);
                ReadRecordResult::from_bytes(&response_bytes)
            }
            .unwrap();

            // Use the ? operator directly on the result
            match result.into_inner() {
                Ok(ok) => {
                    records.push(ok.record_data().to_vec());
                    record_number += 1;
                }
                Err(err) if err.is_end_of_records() => {
                    // End of records reached, break the loop
                    break;
                }
                Err(err) => {
                    // Propagate other errors
                    return Err(err);
                }
            }
        }

        Ok(records)
    }

    // Use our helper function
    match read_all_records(1) {
        Ok(records) => {
            println!("Read {} records successfully", records.len());
            for (i, record) in records.iter().enumerate() {
                println!("Record {}: {} bytes", i + 1, record.len());
            }
        }
        Err(err) => {
            println!("Error reading records: {}", err);
        }
    }
}
