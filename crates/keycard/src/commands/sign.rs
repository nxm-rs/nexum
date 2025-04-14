use bytes::{Bytes, BytesMut};
use nexum_apdu_globalplatform::constants::status;
use nexum_apdu_macros::apdu_pair;

use super::{CLA_GP, DeriveMode, KeyPath, prepare_derivation_parameters};
use crate::types::Signature;
use nexum_apdu_core::response::error::ResponseError;

apdu_pair! {
    /// SIGN command for Keycard
    pub struct Sign {
        command {
            cla: CLA_GP,
            ins: 0xC0,
            required_security_level: SecurityLevel::encrypted(),

            builders {
                /// Create a SIGN command
                pub fn with(
                    data: &[u8; 32],
                    key_path: &KeyPath,
                    derive_mode: Option<DeriveMode>,
                ) -> Result<Self, crate::Error> {
                    let (p1, path_data) = prepare_derivation_parameters(key_path, derive_mode)?;

                    // Combine data and path
                    let buf = match path_data {
                        Some(path_data) => {
                            let mut buf = BytesMut::with_capacity(data.len() + path_data.len());
                            buf.extend(data);
                            buf.extend(path_data);
                            buf.freeze()
                        }
                        None => Bytes::copy_from_slice(data.as_slice()),
                    };

                    Ok(Self::new(p1, 0x00).with_data(buf).with_le(0))
                }

                /// Sign with pinless path
                pub fn with_pinless(data: &[u8; 32]) -> Self {
                    Self::new(0x03, 0x00).with_data(Bytes::copy_from_slice(data.as_slice()))
                }
            }
        }

        response {
            ok {
                /// Success response
                #[sw(status::SW_NO_ERROR)]
                Success {
                    signature: crate::types::Signature,
                },
            }

            errors {
                /// Conditions not satisfied (e.g. secure channel + verified pin)
                #[sw(status::SW_CONDITIONS_NOT_SATISFIED)]
                #[error("Conditions not satisfied: Require secure channel and verified pin")]
                ConditionsNotSatisfied,

                /// Data is less than 32 bytes
                #[sw(status::SW_WRONG_DATA)]
                #[error("Wrong data: Incorrect length for P1")]
                WrongData,

                /// Referenced data not found
                #[sw(status::SW_REFERENCED_DATA_NOT_FOUND)]
                #[error("Referenced data not found: Pinless path not set")]
                ReferencedDataNotFound,
            }

            custom_parse = |response: &nexum_apdu_core::Response| -> Result<SignOk, SignError> {
                use nexum_apdu_core::ApduResponse;

                match response.status() {
                    status::SW_NO_ERROR => match response.payload() {
                        Some(payload) => Ok(SignOk::Success {
                            signature: Signature::try_from(payload.as_ref())
                                .map_err(|e| nexum_apdu_core::response::error::ResponseError::Message(e.to_string()))?,
                        }),
                        None => Err(ResponseError::Parse("No payload data").into()),
                    },
                    status::SW_CONDITIONS_NOT_SATISFIED => Err(SignError::ConditionsNotSatisfied),
                    status::SW_WRONG_DATA => Err(SignError::WrongData),
                    status::SW_REFERENCED_DATA_NOT_FOUND => Err(SignError::ReferencedDataNotFound),
                    _ => Err(SignError::Unknown{sw1: response.status().sw1, sw2: response.status().sw2}),
                }
            }
        }
    }
}
