use nexum_apdu_globalplatform::constants::status::*;
use nexum_apdu_macros::apdu_pair;

use crate::Keypair;

use super::{CLA_GP, DERIVE_FROM_MASTER, derivation_path_to_bytes};
use coins_bip32::path::DerivationPath;

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
pub enum ExportOption {
    /// Export both private and public key
    PrivateAndPublic = 0x00,
    /// Export only the public key
    PublicKeyOnly = 0x01,
    /// Export extended public key (with chain code)
    ExtendedPublicKey = 0x02,
}

apdu_pair! {
    /// EXPORT KEY command for Keycard
    pub struct ExportKey {
        command {
            cla: CLA_GP,
            ins: 0xC2,
            required_security_level: SecurityLevel::full(),

            builders {
                /// General purpose method (prefer using the more specific builders above)
                pub fn from_path(
                    what: ExportOption,
                    derivation_path: &DerivationPath,
                ) -> Self {
                    Self::new(DERIVE_FROM_MASTER, what as u8)
                        .with_le(0)
                        .with_data(derivation_path_to_bytes(derivation_path))
                }
            }
        }

        response {
            ok {
                /// Success response
                #[sw(SW_NO_ERROR)]
                Success {
                    /// Keypair that has been exported
                    keypair: Keypair,
                }
            }

            errors {
                /// Conditions not satisfied (e.g. secure channel + verified pin)
                #[sw(SW_CONDITIONS_NOT_SATISFIED)]
                #[error("Conditions not satisfied: Require secure channel and verified PIN")]
                ConditionsNotSatisfied,

                /// Incorrect P1/P2: Invalid export option
                #[sw(SW_INCORRECT_P1P2)]
                #[error("Incorrect P1/P2: Invalid export option")]
                IncorrectP1P2,

                /// Wrong Data: Invalid derivation path format
                #[sw(SW_WRONG_DATA)]
                #[error("Wrong Data: Invalid derivation path format")]
                WrongData,
            }

            custom_parse = |response: &nexum_apdu_core::Response| -> Result<ExportKeyOk, ExportKeyError> {
                match response.status() {
                    SW_NO_ERROR => {
                        match response.payload() {
                            Some(payload) => {
                                let keypair = Keypair::try_from(payload.as_ref())
                                    .map_err(|_| Error::ParseError("Unable to parse keypair"))?;
                                Ok(ExportKeyOk::Success{
                                    keypair,
                                })
                            },
                            None => Err(ExportKeyError::WrongData),
                        }
                    },
                    SW_CONDITIONS_NOT_SATISFIED => Err(ExportKeyError::ConditionsNotSatisfied),
                    SW_INCORRECT_P1P2 => Err(ExportKeyError::IncorrectP1P2),
                    SW_WRONG_DATA => Err(ExportKeyError::WrongData),
                    _ => Err(ExportKeyError::Unknown {sw1: response.status().sw1, sw2: response.status().sw2}),
                }
            }
        }
    }
}
