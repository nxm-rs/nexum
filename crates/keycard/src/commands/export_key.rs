use nexum_apdu_globalplatform::constants::status::*;
use nexum_apdu_macros::apdu_pair;

use crate::Keypair;

use super::{CLA_GP, DeriveMode, KeyPath, prepare_derivation_parameters};
use coins_bip32::path::DerivationPath;

#[derive(Clone, Copy, Debug)]
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
            required_security_level: SecurityLevel::full(),

            builders {
                /// Export the current key without derivation
                pub fn from_current(what: ExportOption) -> Result<Self, crate::Error> {
                    let command = Self::new(0x00, what as u8).with_le(0);
                    Ok(command)
                }

                /// Export a key derived from the master key
                pub fn from_master(
                    what: ExportOption,
                    path: Option<&DerivationPath>,
                    make_current: bool,
                ) -> Result<Self, crate::Error> {
                    let derive_mode = if make_current {
                        DeriveMode::Persistent
                    } else {
                        DeriveMode::Temporary
                    };

                    let key_path = match path {
                        Some(path) => KeyPath::FromMaster(Some(path.clone())),
                        None => KeyPath::FromMaster(None),
                    };

                    Self::with(what, &key_path, Some(derive_mode))
                }

                /// Export a key derived from the parent key
                pub fn from_parent(
                    what: ExportOption,
                    path: &DerivationPath,
                    make_current: bool,
                ) -> Result<Self, crate::Error> {
                    let derive_mode = if make_current {
                        DeriveMode::Persistent
                    } else {
                        DeriveMode::Temporary
                    };

                    let key_path = KeyPath::FromParent(path.clone());
                    Self::with(what, &key_path, Some(derive_mode))
                }

                /// Export a key derived from the current key
                pub fn from_current_with_derivation(
                    what: ExportOption,
                    path: &DerivationPath,
                    make_current: bool,
                ) -> Result<Self, crate::Error> {
                    let derive_mode = if make_current {
                        DeriveMode::Persistent
                    } else {
                        DeriveMode::Temporary
                    };

                    let key_path = KeyPath::FromCurrent(path.clone());
                    Self::with(what, &key_path, Some(derive_mode))
                }

                /// General purpose method (prefer using the more specific builders above)
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
