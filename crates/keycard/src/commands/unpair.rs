use nexum_apdu_globalplatform::constants::status::*;
use nexum_apdu_macros::apdu_pair;

use super::CLA_GP;

apdu_pair! {
    /// UNPAIR command for Keycard
    pub struct Unpair {
        command {
            cla: CLA_GP,
            ins: 0x13,
            required_security_level: SecurityLevel::auth_mac(),

            builders {
                /// Create an UNPAIR for the nominated index
                pub fn with_index(index: u8) -> Self {
                    Self::new(index, 0x00)
                }
            }
        }

        response {
            ok {
                /// Success response
                #[sw(SW_NO_ERROR)]
                Success
            }

            errors {
                /// Security status not satisfied
                #[sw(SW_SECURITY_STATUS_NOT_SATISFIED)]
                #[error("Security status not satisfied")]
                SecurityStatusNotSatisfied,

                /// Incorrect P1/P2: Index is higher than possible pairing index
                #[sw(SW_INCORRECT_P1P2)]
                #[error("Incorrect P1/P2: Index is higher than possible pairing index")]
                IncorrectP1P2
            }
        }
    }
}
