//! EXTERNAL AUTHENTICATE command for GlobalPlatform
//!
//! This command is used to authenticate the host to the card
//! and establish a secure channel.

use apdu_macros::apdu_pair;

use crate::{
    Error as GpError, Result,
    constants::{cla, external_auth_p1, ins, status},
    crypto::{NULL_BYTES_8, append_des_padding, mac_3des},
};

apdu_pair! {
    /// EXTERNAL AUTHENTICATE command for GlobalPlatform
    pub struct ExternalAuthenticate {
        command {
            cla: cla::MAC,
            ins: ins::EXTERNAL_AUTHENTICATE,
            secure: false,

            builders {
                /// Create a new EXTERNAL AUTHENTICATE command with host cryptogram
                pub fn with_host_cryptogram(host_cryptogram: impl Into<bytes::Bytes>) -> Self {
                    Self::new(external_auth_p1::CMAC, 0x00).with_data(host_cryptogram.into())
                }

                /// Create host cryptogram and command for SCP02
                pub fn from_challenges(
                    enc_key: &[u8],
                    card_challenge: &[u8],
                    host_challenge: &[u8],
                ) -> Result<Self> {
                    let host_cryptogram = Self::calculate_host_cryptogram(
                        enc_key, card_challenge, host_challenge)?;
                    Ok(Self::with_host_cryptogram(host_cryptogram))
                }

                /// Create EXTERNAL AUTHENTICATE with specific security level
                pub fn with_security_level(
                    enc_key: &[u8],
                    card_challenge: &[u8],
                    host_challenge: &[u8],
                    security_level: u8,
                ) -> Result<Self> {
                    let host_cryptogram = Self::calculate_host_cryptogram(
                        enc_key, card_challenge, host_challenge)?;
                    Ok(Self::new(security_level, 0x00).with_data(host_cryptogram))
                }
            }
        }

        response {
            variants {
                /// Success response (9000)
                #[sw(status::SUCCESS)]
                Success,

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
        }
    }
}

impl ExternalAuthenticateCommand {
    /// Calculate the host cryptogram for SCP02
    ///
    /// The host cryptogram is calculated as MAC(card_challenge || host_challenge)
    pub fn calculate_host_cryptogram(
        enc_key: &[u8],
        card_challenge: &[u8],
        host_challenge: &[u8],
    ) -> Result<Vec<u8>> {
        // Validate inputs
        if enc_key.len() != 16 {
            return Err(GpError::InvalidLength {
                expected: 16,
                actual: enc_key.len(),
            });
        }

        if card_challenge.len() != 8 {
            return Err(GpError::InvalidLength {
                expected: 8,
                actual: card_challenge.len(),
            });
        }

        if host_challenge.len() != 8 {
            return Err(GpError::InvalidLength {
                expected: 8,
                actual: host_challenge.len(),
            });
        }

        // Build data: card_challenge || host_challenge
        let mut data = Vec::with_capacity(16);
        data.extend_from_slice(card_challenge);
        data.extend_from_slice(host_challenge);

        // Apply DES padding and calculate MAC
        let padded_data = append_des_padding(&data);
        let cryptogram = mac_3des(enc_key, &padded_data, &NULL_BYTES_8)?;

        Ok(cryptogram.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apdu_core::ApduCommand;
    use hex_literal::hex;

    #[test]
    fn test_calculate_host_cryptogram() {
        let enc_key = hex!("0EF72A1065236DD6CAC718D5E3F379A4");
        let card_challenge = hex!("0076a6c0d55e9535");
        let host_challenge = hex!("266195e638da1b95");

        let cryptogram = ExternalAuthenticateCommand::calculate_host_cryptogram(
            &enc_key,
            &card_challenge,
            &host_challenge,
        )
        .unwrap();

        assert_eq!(cryptogram, hex!("45A5F48DAE68203C"));
    }

    #[test]
    fn test_external_authenticate_command() {
        let cryptogram = hex!("7702AC6CE46A47F0");
        let cmd = ExternalAuthenticateCommand::with_host_cryptogram(cryptogram.to_vec());

        assert_eq!(cmd.class(), cla::MAC);
        assert_eq!(cmd.instruction(), ins::EXTERNAL_AUTHENTICATE);
        assert_eq!(cmd.p1(), external_auth_p1::CMAC);
        assert_eq!(cmd.p2(), 0x00);
        assert_eq!(cmd.data(), Some(cryptogram.as_ref()));

        // Test command serialization
        let raw = cmd.to_bytes();
        assert_eq!(raw.as_ref(), hex!("8482010008 7702AC6CE46A47F0"));
    }

    #[test]
    fn test_from_challenges() {
        let enc_key = hex!("8D289AFE0AB9C45B1C76DEEA182966F4");
        let card_challenge = hex!("000f3fd65d4d6e45");
        let host_challenge = hex!("cf307b6719bf224d");

        let cmd = ExternalAuthenticateCommand::from_challenges(
            &enc_key,
            &card_challenge,
            &host_challenge,
        )
        .unwrap();

        assert_eq!(cmd.class(), cla::MAC);
        assert_eq!(cmd.instruction(), ins::EXTERNAL_AUTHENTICATE);
        assert_eq!(cmd.p1(), external_auth_p1::CMAC);
        assert_eq!(cmd.p2(), 0x00);

        // The exact cryptogram will depend on the MAC implementation
        assert_eq!(cmd.data().unwrap().len(), 8);
    }

    #[test]
    fn test_external_authenticate_response() {
        // Test successful response
        let response_data = hex!("9000");
        let response = ExternalAuthenticateResponse::from_bytes(&response_data).unwrap();
        assert!(matches!(response, ExternalAuthenticateResponse::Success));

        // Test error response
        let response_data = hex!("6982");
        let response = ExternalAuthenticateResponse::from_bytes(&response_data).unwrap();
        assert!(matches!(
            response,
            ExternalAuthenticateResponse::SecurityConditionNotSatisfied
        ));
    }
}
