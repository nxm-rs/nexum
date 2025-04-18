use bytes::{Bytes, BytesMut};
use nexum_apdu_globalplatform::constants::status::*;
use nexum_apdu_macros::apdu_pair;

use super::{CLA_GP, DERIVE_FROM_MASTER, DERIVE_FROM_PINLESS, derivation_path_to_bytes};
use crate::types::Signature;

use coins_bip32::path::DerivationPath;

apdu_pair! {
    /// SIGN command for Keycard
    pub struct Sign {
        command {
            cla: CLA_GP,
            ins: 0xC0,
            required_security_level: SecurityLevel::auth_mac(),

            builders {
                /// Create a SIGN command
                pub fn with(
                    data: &[u8; 32],
                    path: &DerivationPath,
                ) -> Result<Self, crate::Error> {
                    let path_data = derivation_path_to_bytes(path);

                    // Combine data and path
                    let mut buf = BytesMut::with_capacity(data.len() + path_data.len());
                    buf.extend(data);
                    buf.extend(path_data);

                    Ok(Self::new(DERIVE_FROM_MASTER, 0x00).with_data(buf.freeze()).with_le(0))
                }

                /// Sign with pinless path
                pub fn with_pinless(data: &[u8; 32]) -> Self {
                    Self::new(DERIVE_FROM_PINLESS, 0x00).with_data(Bytes::copy_from_slice(data.as_slice()))
                }
            }
        }

        response {
            ok {
                /// Success response
                #[sw(SW_NO_ERROR)]
                Success {
                    signature: crate::types::Signature,
                },
            }

            errors {
                /// Conditions not satisfied (e.g. secure channel + verified pin)
                #[sw(SW_CONDITIONS_NOT_SATISFIED)]
                #[error("Conditions not satisfied: Require secure channel and verified pin")]
                ConditionsNotSatisfied,

                /// Data is less than 32 bytes
                #[sw(SW_WRONG_DATA)]
                #[error("Wrong data: Incorrect length for P1")]
                WrongData,

                /// Referenced data not found
                #[sw(SW_REFERENCED_DATA_NOT_FOUND)]
                #[error("Referenced data not found: Pinless path not set")]
                ReferencedDataNotFound,
            }

            custom_parse = |response: &nexum_apdu_core::Response| -> Result<SignOk, SignError> {
                match response.status() {
                    SW_NO_ERROR => match response.payload() {
                        Some(payload) => Ok(SignOk::Success {
                            signature: Signature::try_from(payload.as_ref())
                                .map_err(|_| Error::ParseError("Unable to parse signature"))?,
                        }),
                        None => Err(Error::ParseError("No payload data"))?,
                    },
                    SW_CONDITIONS_NOT_SATISFIED => Err(SignError::ConditionsNotSatisfied),
                    SW_WRONG_DATA => Err(SignError::WrongData),
                    SW_REFERENCED_DATA_NOT_FOUND => Err(SignError::ReferencedDataNotFound),
                    _ => Err(SignError::Unknown{sw1: response.status().sw1, sw2: response.status().sw2}),
                }
            }
        }
    }
}
