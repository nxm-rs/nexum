#![allow(missing_docs)]
//! Example of using the apdu_pair macro with the new Result-based API for Read Record command

use bytes::Bytes;
use nexum_apdu_core::ApduCommand;
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

                // Other error
                #[sw(_, _)]
                #[error("Other error: {sw1:02X}{sw2:02X}")]
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

    // Simulate a successful response
    let record_data = [
        0x70, 0x12, 0x5A, 0x08, 0x41, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x5F, 0x24, 0x03,
        0x23, 0x12, 0x31, 0x9F, 0x1F, 0x01, 0x00,
    ];
    let response_bytes = Bytes::from([&record_data[..], &[0x90, 0x00]].concat());

    let response =
        ReadRecordResponse::from_bytes(&response_bytes).expect("Failed to parse response");

    // Traditional way
    if let Some(data) = response.record_data() {
        println!("Record data: {:02X?}", data);
    }

    // New Result-based way
    match response.to_result() {
        Ok(ok) => match ok {
            ReadRecordOk::Success { data } => {
                println!("Record data (via Result): {:02X?}", data);
            }
        },
        Err(err) => {
            println!("Error reading record: {}", err);
        }
    }

    // Function that demonstrates error handling with Result
    fn read_all_records(_sfi: u8) -> Result<Vec<Vec<u8>>, ReadRecordError> {
        let mut records = Vec::new();
        let mut record_number = 1;

        loop {
            // In a real application, this would use an executor
            // For this example, we'll simulate responses

            let response = if record_number <= 3 {
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

                ReadRecordResponse::from_bytes(&response_bytes).map_err(|_| {
                    ReadRecordError::OtherError {
                        sw1: 0x6F,
                        sw2: 0x00,
                    }
                })?
            } else {
                // Simulate end of records for record 4+
                let response_bytes = Bytes::from_static(&[0x6A, 0x83]);

                ReadRecordResponse::from_bytes(&response_bytes).map_err(|_| {
                    ReadRecordError::OtherError {
                        sw1: 0x6F,
                        sw2: 0x00,
                    }
                })?
            };

            // Convert to Result
            let result: ReadRecordResult = response.into();

            match result {
                Ok(ReadRecordOk::Success { data }) => {
                    records.push(data);
                    record_number += 1;
                }
                Err(ReadRecordError::RecordNotFound) | Err(ReadRecordError::EndOfRecords) => {
                    break;
                }
                Err(err) => {
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
