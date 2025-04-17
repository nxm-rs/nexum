use nexum_apdu_globalplatform::constants::status::*;
use nexum_apdu_macros::apdu_pair;

use coins_bip32::path::DerivationPath;

use crate::ApplicationStatus;

use super::CLA_GP;

apdu_pair! {
    /// GET STATUS command for Keycard
    pub struct GetStatus {
        command {
            cla: CLA_GP,
            ins: 0xF2,
            required_security_level: SecurityLevel::mac_protected(),

            builders {
                /// Create a GET STATUS command for the application status.
                pub fn application() -> Self {
                    Self::new(0x00, 0x00).with_le(0x00)
                }

                /// Create a GET STATUS command for the key path status.
                pub fn key_path() -> Self {
                    Self::new(0x01, 0x00).with_le(0x00)
                }
            }
        }

        response {
            ok {
                /// Success response
                #[sw(SW_NO_ERROR)]
                ApplicationStatus {
                    /// The status of the Keycard application
                    status: ApplicationStatus,
                },

                /// Success response
                #[sw(SW_NO_ERROR)]
                KeyPathStatus {
                    path: DerivationPath,
                }
            }

            errors {
                /// Incorrect P1/P2: Undefined P1
                #[sw(status::SW_INCORRECT_P1P2)]
                #[error("Incorrect P1/P2: Undefined P1")]
                IncorrectP1P2,
            }

            custom_parse = |response: &nexum_apdu_core::Response| -> Result<GetStatusOk, GetStatusError> {
                match response.status() {
                    SW_NO_ERROR => {
                        match response.payload() {
                            Some(data) if data.len() % 4 == 0 => {
                                let u32_iter = data.chunks(4).map(|chunk| u32::from_be_bytes(chunk.try_into().unwrap()));
                                let path = DerivationPath::from_iter(u32_iter);
                                Ok(GetStatusOk::KeyPathStatus {
                                    path,
                                })
                            },
                            Some(data) => {
                                let status = ApplicationStatus::try_from(data.as_ref())
                                    .map_err(|_| Error::ParseError("Unable to parse application status"))?;
                                Ok(GetStatusOk::ApplicationStatus {
                                    status,
                                })
                            },
                            _ => Err(GetStatusError::IncorrectP1P2),
                        }
                    }
                    SW_INCORRECT_P1P2 => Err(GetStatusError::IncorrectP1P2),
                    _ => Err(GetStatusError::Unknown { sw1: response.status().sw1, sw2: response.status().sw2 }),
                }
            }
        }
    }
}
