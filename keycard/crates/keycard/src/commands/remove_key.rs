use nexum_apdu_globalplatform::constants::status::*;
use nexum_apdu_macros::apdu_pair;

use super::CLA_GP;

apdu_pair! {
    /// REMOVE KEY command for Keycard
    pub struct RemoveKey {
        command {
            cla: CLA_GP,
            ins: 0xD3,
            required_security_level: SecurityLevel::auth_mac(),

            builders {
                /// Create a REMOVE KEY command
                pub fn remove() -> Self {
                    Self::new(0x00, 0x00)
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
                /// Conditions not satisfied: PIN not verified
                #[sw(SW_CONDITIONS_NOT_SATISFIED)]
                #[error("Conditions not satisfied: PIN not verified")]
                ConditionsNotSatisfied,
            }
        }
    }
}
