use nexum_apdu_globalplatform::constants::status;
use nexum_apdu_macros::apdu_pair;

use crate::Keypair;

use super::{CLA_GP, DeriveMode, KeyPath, prepare_derivation_parameters};

pub enum ExportOption {
    PrivateAndPublic = 0x00,
    PublicKeyOnly = 0x01,
    ExtendedPublicKey = 0x02,
}

apdu_pair! {
    /// EXPORT KEY command for Keycard
    pub struct ExportKey {
        command {
            cla: CLA_GP,
            ins: 0xC2,
            required_security_level: SecurityLevel::authenticated_encrypted(),

            builders {
                /// Create an EXPORT KEY command
                pub fn with(
                    what: ExportOption,
                    key_path: &KeyPath,
                    derive_mode: Option<DeriveMode>,
                ) -> Result<Self, crate::Error> {
                    let (p1, path_data) = prepare_derivation_parameters(key_path, derive_mode)?;

                    let command = Self::new(p1, what as u8).with_le(0);
                    Ok(match path_data {
                        Some(path_data) => command.with_data(path_data),
                        None => command,
                    })
                }
            }
        }

        response {
            ok {
                /// Success response
                #[sw(status::SW_NO_ERROR)]
                Success {
                    keypair: Keypair,
                }
            }

            errors {
                /// Conditions not satisfied (e.g. secure channel + verified pin)
                #[sw(status::SW_CONDITIONS_NOT_SATISFIED)]
                #[error("Conditions not satisfied: Require secure channel and verified PIN")]
                ConditionsNotSatisfied,

                /// Incorrect P1/P2: Invalid export option
                #[sw(status::SW_INCORRECT_P1P2)]
                #[error("Incorrect P1/P2: Invalid export option")]
                IncorrectP1P2,

                /// Wrong Data: Invalid derivation path format
                #[sw(status::SW_WRONG_DATA)]
                #[error("Wrong Data: Invalid derivation path format")]
                WrongData,
            }

            custom_parse = |response: &nexum_apdu_core::Response| -> Result<ExportKeyOk, ExportKeyError> {
                use nexum_apdu_core::ApduResponse;

                match response.status() {
                    status::SW_NO_ERROR => {
                        match response.payload() {
                            Some(payload) => {
                                let keypair = Keypair::try_from(payload.as_ref())
                                    .map_err(|e| nexum_apdu_core::response::error::ResponseError::Message(e.to_string()))?;
                                Ok(ExportKeyOk::Success{
                                    keypair,
                                })
                            },
                            None => Err(ExportKeyError::WrongData),
                        }
                    },
                    status::SW_CONDITIONS_NOT_SATISFIED => Err(ExportKeyError::ConditionsNotSatisfied),
                    status::SW_INCORRECT_P1P2 => Err(ExportKeyError::IncorrectP1P2),
                    status::SW_WRONG_DATA => Err(ExportKeyError::WrongData),
                    _ => Err(ExportKeyError::Unknown {sw1: response.status().sw1, sw2: response.status().sw2}),
                }
            }
        }
    }
}
