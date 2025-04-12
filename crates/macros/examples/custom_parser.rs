//! Example demonstrating a custom parser with the new Result-based API

use bytes::Bytes;
use nexum_apdu_core::prelude::*;
use nexum_apdu_macros::apdu_pair;

apdu_pair! {
    /// Verify PIN command
    pub struct VerifyPin {
        command {
            cla: 0x00,
            ins: 0x20,

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
            }

            // Custom parser now takes the complete Response and returns a Result<OkEnum, ErrorEnum>
            custom_parse = |response: &nexum_apdu_core::Response| -> Result<VerifyPinOk, VerifyPinError> {
                use nexum_apdu_core::ApduResponse;

                let status = response.status();
                let sw1 = status.sw1;
                let sw2 = status.sw2;
                let payload = response.payload();
                let payload_bytes = payload.as_ref().map(|b| b.as_ref()).unwrap_or(&[]);

                match (sw1, sw2) {
                    (0x90, 0x00) => {
                        // PIN verification successful
                        Ok(VerifyPinOk::Verified)
                    },
                    (0x69, 0x83) => {
                        // PIN blocked
                        Err(VerifyPinError::Blocked)
                    },
                    (0x63, sw2) if (sw2 & 0xF0) == 0xC0 => {
                        // PIN incorrect, extract counter from lower nibble of SW2
                        let count = sw2 & 0x0F;

                        // When attempting verification, count is attempts remaining for error
                        if !payload_bytes.is_empty() {
                            Err(VerifyPinError::Incorrect { count })
                        } else {
                            // When querying, we want to return success with attempts count
                            Ok(VerifyPinOk::AttemptsRemaining { count })
                        }
                    },
                    (sw1, sw2) => {
                        // Other status words
                        Err(VerifyPinError::Unknown { sw1, sw2 })
                    }
                }
            }
        }
    }
}
// Implement methods directly on the generated types
impl VerifyPinOk {
    /// Get attempts remaining (if available)
    pub fn attempts_remaining(&self) -> Option<u8> {
        match self {
            Self::AttemptsRemaining { count } => Some(*count),
            _ => None,
        }
    }

    /// Check if verification was successful
    pub fn is_verified(&self) -> bool {
        matches!(self, Self::Verified)
    }
}

impl VerifyPinError {
    /// Get attempts remaining (if available)
    pub fn attempts_remaining(&self) -> Option<u8> {
        match self {
            Self::Incorrect { count } => Some(*count),
            _ => None,
        }
    }

    /// Check if PIN is blocked
    pub fn is_blocked(&self) -> bool {
        matches!(self, Self::Blocked)
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
    fn verify_pin(pin: &[u8], attempts_left: u8) -> Result<VerifyPinOk, VerifyPinError> {
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

        // Parse and return the result directly (no longer need to unwrap)
        VerifyPinCommand::parse_response_raw(response_bytes)
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
            if let Some(count) = err.attempts_remaining() {
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

            if err.is_blocked() {
                println!("PIN is blocked, please reset your card or contact support");
            }
        }
    }

    // Function that uses our new API with question mark operator
    fn authenticate_user(pin: &[u8]) -> Result<(), VerifyPinError> {
        // First check if PIN is blocked by querying remaining attempts
        let query_result = VerifyPinCommand::parse_response_raw(Bytes::from_static(&[0x63, 0xC2]))?;

        // Get the inner result - now more ergonomic with deref
        match query_result {
            VerifyPinOk::AttemptsRemaining { count } => {
                println!("PIN attempts remaining: {}", count);
                if count == 0 {
                    return Err(VerifyPinError::ResponseError(
                        nexum_apdu_core::response::error::ResponseError::Message(
                            "PIN is blocked".to_string(),
                        ),
                    ));
                }
            }
            _ => {
                // Unexpected response, continue anyway
            }
        }

        // Now try to verify the PIN - more ergonomic from_bytes accepting any AsRef<[u8]>
        let success = Bytes::from_static(&[0x90, 0x00]);
        let fail = Bytes::from_static(&[0x63, 0xC1]);
        let verify_ok = VerifyPinCommand::parse_response_raw(if pin == [0x31, 0x32, 0x33, 0x34] {
            success
        } else {
            fail // 1 attempt left
        })?;

        // Process success
        match verify_ok {
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
