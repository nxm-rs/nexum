use k256::{PublicKey, elliptic_curve::sec1::ToEncodedPoint};
use nexum_apdu_globalplatform::constants::status::*;
use nexum_apdu_macros::apdu_pair;

use super::CLA_GP;

use crate::crypto::{Challenge, KeycardScp};
use cipher::Iv;

apdu_pair! {
    /// OPEN SECURE CHANNEL command for Keycard
    pub struct OpenSecureChannel {
        command {
            cla: CLA_GP,
            ins: 0x10,

            builders {
                /// Create an OPEN SECURE CHANNEL command with parameters
                pub fn with_pairing_index_and_pubkey(pairing_index: u8, public_key: &PublicKey) -> Self {
                    Self::new(pairing_index, 0x00).with_data(public_key.to_encoded_point(false).to_bytes()).with_le(0)
                }
            }
        }

        response {
            ok {
                /// Success response
                #[sw(SW_NO_ERROR)]
                Success {
                    challenge: Challenge,
                    iv: Iv::<KeycardScp>,
                },
            }

            errors {
                /// Incorrect P1/P2: Invalid pairing index
                #[sw(SW_INCORRECT_P1P2)]
                #[error("Incorrect P1/P2: Invalid pairing index")]
                IncorrectP1P2,

                /// Wrong data: Data is not a public key
                #[sw(SW_WRONG_DATA)]
                #[error("Wrong data: Data is not a public key")]
                WrongData,

                /// MAC cannot be verified
                #[sw(SW_SECURITY_STATUS_NOT_SATISFIED)]
                #[error("Security status not satisfied: MAC cannot be verified")]
                SecurityStatusNotSatisfied,
            }

            custom_parse = |response: &nexum_apdu_core::Response| -> Result<OpenSecureChannelOk, OpenSecureChannelError> {
                match response.status() {
                    SW_NO_ERROR => {
                        match response.payload() {
                            Some(payload) => {
                                if payload.len() != std::mem::size_of::<Challenge>() + std::mem::size_of::<Iv<KeycardScp>>() {
                                    return Err(Error::ParseError("Invalid payload length"))?;
                                }
                                let challenge = Challenge::from_slice(&payload[..std::mem::size_of::<Challenge>()]);
                                let iv = Iv::<KeycardScp>::from_slice(&payload[std::mem::size_of::<Challenge>()..]);
                                Ok(OpenSecureChannelOk::Success { challenge: *challenge, iv: *iv })
                            }
                            None => Err(Error::ParseError("No payload"))?,
                        }
                    }
                    SW_INCORRECT_P1P2 => Err(OpenSecureChannelError::IncorrectP1P2),
                    SW_WRONG_DATA => Err(OpenSecureChannelError::WrongData),
                    SW_SECURITY_STATUS_NOT_SATISFIED => Err(OpenSecureChannelError::SecurityStatusNotSatisfied),
                    _ => Err(OpenSecureChannelError::Unknown { sw1: response.status().sw1, sw2: response.status().sw2 }),
                }
            }
        }
    }
}
