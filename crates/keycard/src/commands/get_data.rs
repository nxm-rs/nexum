use nexum_apdu_globalplatform::constants::status::*;
use nexum_apdu_macros::apdu_pair;

use super::{CLA_GP, PersistentRecord};

apdu_pair! {
    /// GET DATA command for Keycard
    pub struct GetData {
        command {
            cla: CLA_GP,
            ins: 0xCA,

            builders {
                /// Create a GET DATA command as a request for the specified record.
                pub fn get(record: PersistentRecord) -> Self {
                    Self::new(record as u8, 0x00).with_le(0)
                }
            }
        }

        response {
            ok {
                /// Success response
                #[sw(SW_NO_ERROR)]
                #[payload(field = "data")]
                Success {
                    /// The data retrieved from the specified record.
                    data: Vec<u8>,
                }
            }

            errors {
                /// Incorrect P1/P2: The record specified is not valid
                #[sw(SW_INCORRECT_P1P2)]
                #[error("Incorrect P1/P2: The record specified is not valid")]
                IncorrectP1P2,
            }

            custom_parse = |response: &nexum_apdu_core::Response| -> Result<GetDataOk, GetDataError> {
                match response.status() {
                    SW_NO_ERROR => Ok(GetDataOk::Success { data: response.payload().as_ref().unwrap_or(&Bytes::new()).to_vec() }),
                    SW_INCORRECT_P1P2 => Err(GetDataError::IncorrectP1P2),
                    _ => Err(GetDataError::Unknown { sw1: response.status().sw1, sw2: response.status().sw2 }),
                }
            }
        }
    }
}
