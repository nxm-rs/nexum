use crate::Challenge;
use nexum_apdu_core::response::error::ResponseError;
use nexum_apdu_globalplatform::constants::status;
use nexum_apdu_macros::apdu_pair;

use crate::crypto::Cryptogram;

use super::CLA_GP;

apdu_pair! {
    /// MUTUALLY AUTHENTICATE command for Keycard
    pub struct MutuallyAuthenticate {
        command {
            cla: CLA_GP,
            ins: 0x11,

            builders {
                /// Create a MUTUALLY AUTHENTICATE command with challenge
                pub fn with_challenge(challenge: &Challenge) -> Self {
                    Self::new(0x00, 0x00).with_data(challenge.to_vec()).with_le(0)
                }
            }
        }

        response {
            ok {
                /// Success response
                #[sw(status::SW_NO_ERROR)]
                Success {
                    cryptogram: Cryptogram,
                },
            }

            errors {
                /// Previous command was not OPEN SECURE CHANNEL
                #[sw(status::SW_CONDITIONS_NOT_SATISFIED)]
                #[error("Conditions not satisfied: Previous command was not OPEN SECURE CHANNEL")]
                ConditionsNotSatisfied,

                /// Client cryptogram verification fails
                #[sw(status::SW_SECURITY_STATUS_NOT_SATISFIED)]
                #[error("Security status not satisfied: Client cryptogram verification failed")]
                SecurityStatusNotSatisfied,
            }

            custom_parse = |response: &nexum_apdu_core::Response| -> Result<MutuallyAuthenticateOk, MutuallyAuthenticateError> {
                use nexum_apdu_core::ApduResponse;

                match response.status() {
                    status::SW_NO_ERROR => {
                        match response.payload() {
                            Some(payload) => {
                                if payload.len() != 32 {
                                    return Err(ResponseError::Parse("Invalid payload length").into());
                                }
                                let cryptogram = Cryptogram::from_slice(payload);
                                Ok(MutuallyAuthenticateOk::Success { cryptogram: *cryptogram })
                            },
                            None => Err(ResponseError::Parse("No payload").into()),
                        }
                    },
                    status::SW_CONDITIONS_NOT_SATISFIED => Err(MutuallyAuthenticateError::ConditionsNotSatisfied),
                    status::SW_SECURITY_STATUS_NOT_SATISFIED => Err(MutuallyAuthenticateError::SecurityStatusNotSatisfied),
                    _ => Err(MutuallyAuthenticateError::Unknown { sw1: response.status().sw1, sw2: response.status().sw2 }),
                }
            }
        }
    }
}
