//! Example demonstrating a custom parser with the new Result-based API

use bytes::Bytes;
use nexum_apdu_core::{ApduCommand, Error as ApduError, StatusWord};
use nexum_apdu_macros::apdu_pair;

apdu_pair! {
    /// Verify PIN command
    pub struct VerifyPin {
        command {
            cla: 0x00,
            ins: 0x20,
            secure: false,

            builders {
                /// Verify PIN with the given data
                pub fn with_pin(pin: &[u8]) -> Self {
                    Self::new(0x00, 0x00).with_data(pin.to_vec())
                }

                /// Query remaining PIN attempts
                pub fn query_remaining() -> Self {
                    Self::new(0x00, 0x00)
                }
            }
        }

        response {
            ok {
                // PIN verification successful
                #[sw(0x90, 0x00)]
                Verified,

                // PIN attempts remaining
                #[sw(0x63, _)]
                AttemptsRemaining {
                    count: u8,
                }
            }

            errors {
                // PIN blocked
                #[sw(0x69, 0x83)]
                #[error("PIN is blocked")]
                Blocked,

                // PIN incorrect
                #[sw(0x63, _)]
                #[error("PIN incorrect, {count} attempts remaining")]
                Incorrect {
                    count: u8,
                },

                // Other errors
                #[sw(_, _)]
                #[error("Other error: {sw1:02X}{sw2:02X}")]
                Other {
                    sw1: u8,
                    sw2: u8,
                }
            }

            // Custom parser to handle special cases like attempts remaining
            custom_parse = |payload: &[u8], sw: StatusWord| -> Result<VerifyPinResponse, nexum_apdu_core::response::error::ResponseError> {
                match (sw.sw1, sw.sw2) {
                    (0x90, 0x00) => {
                        // PIN verification successful
                        Ok(VerifyPinResponse::Verified)
                    },
                    (0x69, 0x83) => {
                        // PIN blocked
                        Ok(VerifyPinResponse::Blocked)
                    },
                    (0x63, sw2) if (sw2 & 0xF0) == 0xC0 => {
                        // PIN incorrect, extract counter from lower nibble of SW2
                        let count = sw2 & 0x0F;

                        // When attempting verification, count is attempts remaining for error
                        if !payload.is_empty() {
                            Ok(VerifyPinResponse::Incorrect { count })
                        } else {
                            // When querying, we want to return success with attempts count
                            Ok(VerifyPinResponse::AttemptsRemaining { count })
                        }
                    },
                    (sw1, sw2) => {
                        // Other status words
                        Ok(VerifyPinResponse::Other { sw1, sw2 })
                    }
                }
            }

            methods {
                /// Get attempts remaining (if available)
                pub fn attempts_remaining(&self) -> Option<u8> {
                    match self {
                        Self::AttemptsRemaining { count } => Some(*count),
                        Self::Incorrect { count } => Some(*count),
                        _ => None,
                    }
                }

                /// Check if verification was successful
                pub fn is_verified(&self) -> bool {
                    matches!(self, Self::Verified)
                }

                /// Check if PIN is blocked
                pub fn is_blocked(&self) -> bool {
                    matches!(self, Self::Blocked)
                }
            }
        }
    }
}

fn main() {
    // Example usage
    let pin = [0x31, 0x32, 0x33, 0x34]; // ASCII '1234'
    let verify_cmd = VerifyPinCommand::with_pin(&pin);

    println!(
        "Verify PIN command: CLA={:#04x}, INS={:#04x}, P1={:#04x}, P2={:#04x}",
        verify_cmd.class(),
        verify_cmd.instruction(),
        verify_cmd.p1(),
        verify_cmd.p2()
    );

    // Function that demonstrates converting between response types
    fn verify_pin(pin: &[u8], attempts_left: u8) -> VerifyPinResult {
        // Simulate different responses based on the input
        let response_bytes = if pin == [0x31, 0x32, 0x33, 0x34] {
            // Correct PIN: 1234
            Bytes::from_static(&[0x90, 0x00])
        } else if attempts_left > 1 {
            // Incorrect PIN, still have attempts
            Bytes::copy_from_slice(&[0x63, 0xC0 | (attempts_left - 1)])
        } else {
            // PIN blocked
            Bytes::from_static(&[0x69, 0x83])
        };

        // Parse and convert to Result
        let response =
            VerifyPinResponse::from_bytes(&response_bytes).expect("Failed to parse response");

        response.into()
    }

    // Try with correct PIN
    match verify_pin(&[0x31, 0x32, 0x33, 0x34], 3) {
        Ok(ok) => match ok {
            VerifyPinOk::Verified => {
                println!("PIN verified successfully!");
            }
            VerifyPinOk::AttemptsRemaining { count } => {
                println!("PIN not verified, {} attempts remaining", count);
            }
        },
        Err(err) => {
            println!("Error: {}", err);
        }
    }

    // Try with incorrect PIN
    match verify_pin(&[0x35, 0x36, 0x37, 0x38], 3) {
        Ok(ok) => match ok {
            VerifyPinOk::Verified => {
                println!("PIN verified successfully!");
            }
            VerifyPinOk::AttemptsRemaining { count } => {
                println!("PIN not verified, {} attempts remaining", count);
            }
        },
        Err(err) => {
            println!("Error: {}", err);

            // We can match on specific error variants
            if let VerifyPinError::Incorrect { count } = err {
                println!("Incorrect PIN, {} attempts remaining", count);
            }
        }
    }

    // Try with PIN blocked
    match verify_pin(&[0x35, 0x36, 0x37, 0x38], 1) {
        Ok(_) => {
            println!("PIN verified successfully!");
        }
        Err(err) => {
            println!("Error: {}", err);

            if let VerifyPinError::Blocked = err {
                println!("PIN is blocked, please reset your card or contact support");
            }
        }
    }

    // Function that uses our new API with question mark operator
    fn authenticate_user(pin: &[u8]) -> Result<(), ApduError> {
        // In a real application, this would use an actual card executor
        // For this example, we'll simulate responses

        // First check if PIN is blocked by querying remaining attempts
        let query_response = VerifyPinResponse::from_bytes(&[0x63, 0xC2]) // Simulate 2 attempts left
            .map_err(|e| ApduError::Response(e))?;

        let query_result: VerifyPinResult = query_response.into();

        match query_result {
            Ok(VerifyPinOk::AttemptsRemaining { count }) => {
                println!("PIN attempts remaining: {}", count);
                if count == 0 {
                    return Err(ApduError::other("PIN is blocked"));
                }
            }
            _ => {
                // Unexpected response, continue anyway
            }
        }

        // Now try to verify the PIN
        let verify_response = VerifyPinResponse::from_bytes(if pin == [0x31, 0x32, 0x33, 0x34] {
            &[0x90, 0x00] // Success
        } else {
            &[0x63, 0xC1] // 1 attempt left
        })
        .map_err(|e| ApduError::Response(e))?;

        // Convert to Result
        let verify_result: VerifyPinResult = verify_response.into();

        // Use ? to propagate errors
        match verify_result? {
            VerifyPinOk::Verified => {
                println!("PIN verification successful");
                Ok(())
            }
            VerifyPinOk::AttemptsRemaining { count } => {
                println!("PIN verification not needed, {} attempts remaining", count);
                Ok(())
            }
        }
    }

    // Use our authenticate function
    match authenticate_user(&[0x31, 0x32, 0x33, 0x34]) {
        Ok(()) => {
            println!("Authentication successful");
        }
        Err(err) => {
            println!("Authentication failed: {}", err);
        }
    }
}
