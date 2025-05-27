//! INITIALIZE UPDATE command for GlobalPlatform
//!
//! This command is used to start a secure channel session.

use nexum_apdu_macros::apdu_pair;

use crate::constants::*;

apdu_pair! {
    /// INITIALIZE UPDATE command for GlobalPlatform
    pub struct InitializeUpdate {
        command {
            cla: cla::GP,
            ins: ins::INITIALIZE_UPDATE,

            builders {
                /// Create a new INITIALIZE UPDATE command with a host challenge
                pub fn with_challenge(host_challenge: impl Into<bytes::Bytes>) -> Self {
                    Self::new(0x00, 0x00).with_data(host_challenge.into()).with_le(0)
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
            ok {
                /// Success response
                #[sw(SW_NO_ERROR)]
                Success {
                    /// Key diversification data
                    key_diversification_data: [u8; 10],
                    /// Key information
                    key_info: [u8; 2],
                    /// Sequence counter
                    sequence_counter: [u8; 2],
                    /// Card challenge
                    card_challenge: [u8; 6],
                    /// Card cryptogram
                    card_cryptogram: [u8; 8],
                },
            }

            errors {
                /// Security status not satisfied
                #[sw(SW_SECURITY_STATUS_NOT_SATISFIED)]
                #[error("Security status not satisfied")]
                SecurityStatusNotSatisfied,
            }

            custom_parse = |response: &nexum_apdu_core::Response| -> Result<InitializeUpdateOk, InitializeUpdateError> {
                let status = response.status();
                let sw1 = status.sw1;
                let sw2 = status.sw2;

                match status {
                    SW_NO_ERROR => {
                        if let Some(payload) = response.payload() {
                            if payload.len() == 28 {
                                // Key diversification data (10 bytes)
                                let key_diversification_data: [u8; 10] = payload[0..10].try_into().unwrap();

                                // Key information (2 bytes)
                                let key_info: [u8; 2] = payload[10..12].try_into().unwrap();

                                // Sequence counter (2 bytes)
                                let sequence_counter: [u8; 2] = payload[12..14].try_into().unwrap();

                                // Card challenge (6 bytes)
                                let card_challenge: [u8; 6] = payload[14..20].try_into().unwrap();

                                // Card cryptogram (8 bytes)
                                let card_cryptogram: [u8; 8] = payload[20..28].try_into().unwrap();

                                return Ok(InitializeUpdateOk::Success {
                                    key_diversification_data,
                                    key_info,
                                    sequence_counter,
                                    card_challenge,
                                    card_cryptogram,
                                })
                            }
                        }
                        Err(nexum_apdu_core::Error::parse("Response data incorrect length").into())
                    }
                    SW_SECURITY_STATUS_NOT_SATISFIED => Err(InitializeUpdateError::SecurityStatusNotSatisfied),
                    _ => Err(InitializeUpdateError::Unknown {
                        sw1,
                        sw2
                    }),
                }
            }
        }
    }
}

impl InitializeUpdateOk {
    /// Get the SCP version
    pub const fn scp_version(&self) -> Option<u8> {
        match self {
            Self::Success { key_info, .. } => {
                if key_info.len() >= 2 {
                    Some(key_info[1])
                } else {
                    None
                }
            }
        }
    }

    /// Get the key version number
    pub const fn key_version_number(&self) -> Option<u8> {
        match self {
            Self::Success { key_info, .. } => match !key_info.is_empty() {
                true => Some(key_info[0]),
                false => None,
            },
        }
    }

    /// Get the sequence counter
    pub const fn sequence_counter(&self) -> Option<&[u8]> {
        match self {
            Self::Success {
                sequence_counter, ..
            } => Some(sequence_counter),
        }
    }

    /// Get the security level supported by the card
    pub const fn security_level(&self) -> Option<u8> {
        match self {
            Self::Success { key_info, .. } => match key_info.len() >= 2 {
                true => Some(key_info[1]),
                false => None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
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

        // Test command serialization
        let raw = cmd.to_bytes();
        assert_eq!(raw.as_ref(), hex!("805000000301020300"));
    }

    #[test]
    fn test_initialize_update_response() {
        // Test successful response
        let response_data = Bytes::from_static(&hex!(
            "000002650183039536622002000de9c62ba1c4c8e55fcb91b6654ce49000"
        ));

        let result = InitializeUpdateCommand::parse_response_raw(response_data).unwrap();

        assert!(matches!(result, InitializeUpdateOk::Success { .. }));
        assert_eq!(result.scp_version(), Some(0x02));
        assert_eq!(result.key_version_number(), Some(0x20));

        // Check sequence counter using the sequence_counter() method
        if let Some(counter) = result.sequence_counter() {
            assert_eq!(counter, &[0x00, 0x0D]);
        } else {
            panic!("Sequence counter should be present");
        }

        match result {
            InitializeUpdateOk::Success {
                key_diversification_data,
                key_info,
                sequence_counter,
                card_challenge,
                card_cryptogram,
            } => {
                assert_eq!(key_diversification_data, hex!("00000265018303953662"));
                assert_eq!(key_info, hex!("2002"));
                assert_eq!(sequence_counter, hex!("000D"));
                assert_eq!(card_challenge, hex!("E9C62BA1C4C8"));
                assert_eq!(card_cryptogram, hex!("E55FCB91B6654CE4"));
            }
        }

        // Test error response
        let response_data = Bytes::from_static(&hex!("6982"));
        let result = InitializeUpdateCommand::parse_response_raw(response_data).unwrap_err();
        assert!(matches!(
            result,
            InitializeUpdateError::SecurityStatusNotSatisfied
        ));
    }
}
