//! INITIALIZE UPDATE command for GlobalPlatform
//!
//! This command is used to start a secure channel session.

use nexum_apdu_macros::apdu_pair;

use crate::constants::{cla, ins, status};

apdu_pair! {
    /// INITIALIZE UPDATE command for GlobalPlatform
    pub struct InitializeUpdate {
        command {
            cla: cla::GP,
            ins: ins::INITIALIZE_UPDATE,
            secure: false,

            builders {
                /// Create a new INITIALIZE UPDATE command with a host challenge
                pub fn with_challenge(host_challenge: impl Into<bytes::Bytes>) -> Self {
                    Self::new(0x00, 0x00).with_data(host_challenge.into()).with_le(0x00)
                }

                /// Create a new INITIALIZE UPDATE command with random host challenge
                pub fn with_random_challenge() -> Self {
                    let mut challenge = [0u8; 8];
                    rand::RngCore::fill_bytes(&mut rand::rng(), &mut challenge);
                    Self::with_challenge(challenge.to_vec())
                }
            }
        }

        response {
            variants {
                /// Success response (9000)
                #[sw(status::SUCCESS)]
                Success {
                    key_diversification_data: [u8; 10],
                    key_info: [u8; 2],
                    sequence_counter: [u8; 2],
                    card_challenge: [u8; 6],
                    card_cryptogram: [u8; 8],
                },

                /// Security condition not satisfied (6982)
                #[sw(status::SECURITY_CONDITION_NOT_SATISFIED)]
                SecurityConditionNotSatisfied,

                /// Authentication method blocked (6983)
                #[sw(status::AUTHENTICATION_METHOD_BLOCKED)]
                AuthenticationMethodBlocked,

                /// Other error
                #[sw(_, _)]
                OtherError {
                    sw1: u8,
                    sw2: u8,
                }
            }

            parse_payload = |payload: &[u8], _sw: nexum_apdu_core::StatusWord, variant: &mut Self| -> Result<(), nexum_apdu_core::Error> {
                if let Self::Success {
                    key_diversification_data,
                    key_info,
                    sequence_counter,
                    card_challenge,
                    card_cryptogram
                } = variant {
                    if payload.len() != 28 {
                        return Err(nexum_apdu_core::Error::Parse("Response data incorrect length"));
                    }

                    // Key diversification data (10 bytes)
                    key_diversification_data.copy_from_slice(&payload[0..10]);

                    // Key information (2 bytes)
                    key_info.copy_from_slice(&payload[10..12]);

                    // Sequence counter (2 bytes)
                    sequence_counter.copy_from_slice(&payload[12..14]);

                    // Card challenge (6 bytes)
                    card_challenge.copy_from_slice(&payload[14..20]);

                    // Card cryptogram (8 bytes)
                    card_cryptogram.copy_from_slice(&payload[20..28]);
                }

                Ok(())
            }

            methods {
                /// Get the SCP version
                pub const fn scp_version(&self) -> Option<u8> {
                    match self {
                        Self::Success { key_info, .. } => {
                            if key_info.len() >= 2 {
                                Some(key_info[1])
                            } else {
                                None
                            }
                        },
                        _ => None,
                    }
                }

                /// Get the key version number
                pub const fn key_version_number(&self) -> Option<u8> {
                    match self {
                        Self::Success { key_info, .. } => {
                            match !key_info.is_empty() {
                                true => Some(key_info[0]),
                                false => None,
                            }
                        },
                        _ => None,
                    }
                }

                /// Get the sequence counter
                pub const fn sequence_counter(&self) -> Option<&[u8]> {
                    match self {
                        Self::Success { sequence_counter, .. } => {
                            Some(sequence_counter)
                        },
                        _ => None,
                    }
                }

                /// Get the security level supported by the card
                pub const fn security_level(&self) -> Option<u8> {
                    match self {
                        Self::Success { key_info, .. } => {
                            match key_info.len() >= 2 {
                                true => Some(key_info[1]),
                                false => None,
                            }
                        },
                        _ => None,
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;
    use nexum_apdu_core::ApduCommand;

    #[test]
    fn test_initialize_update_command() {
        // Test with specific challenge
        let challenge = hex!("010203");
        let cmd = InitializeUpdateCommand::with_challenge(challenge.to_vec());

        assert_eq!(cmd.class(), cla::GP);
        assert_eq!(cmd.instruction(), ins::INITIALIZE_UPDATE);
        assert_eq!(cmd.p1(), 0x00);
        assert_eq!(cmd.p2(), 0x00);
        assert_eq!(cmd.data(), Some(challenge.as_ref()));
        assert_eq!(cmd.expected_length(), Some(0x00));

        // Test command serialization
        let raw = cmd.to_bytes();
        assert_eq!(raw.as_ref(), hex!("805000000301020300"));
    }

    #[test]
    fn test_initialize_update_response() {
        // Test successful response
        let response_data = hex!("000002650183039536622002000de9c62ba1c4c8e55fcb91b6654ce49000");

        let response = InitializeUpdateResponse::from_bytes(&response_data).unwrap();

        assert!(matches!(response, InitializeUpdateResponse::Success { .. }));
        assert_eq!(response.scp_version(), Some(0x02));
        assert_eq!(response.key_version_number(), Some(0x20));

        // Check sequence counter using the sequence_counter() method
        if let Some(counter) = response.sequence_counter() {
            assert_eq!(counter, &[0x00, 0x0D]);
        } else {
            panic!("Sequence counter should be present");
        }

        if let InitializeUpdateResponse::Success {
            key_diversification_data,
            key_info,
            sequence_counter,
            card_challenge,
            card_cryptogram,
        } = response
        {
            // Use the correct size hex literals for each field
            assert_eq!(key_diversification_data, hex!("00000265018303953662"));
            assert_eq!(key_info, hex!("2002"));
            assert_eq!(sequence_counter, hex!("000D"));
            assert_eq!(card_challenge, hex!("E9C62BA1C4C8"));
            assert_eq!(card_cryptogram, hex!("E55FCB91B6654CE4"));
        }

        // Test error response
        let response_data = hex!("6982");
        let response = InitializeUpdateResponse::from_bytes(&response_data).unwrap();
        assert!(matches!(
            response,
            InitializeUpdateResponse::SecurityConditionNotSatisfied
        ));
    }
}
