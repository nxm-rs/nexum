use nexum_apdu_globalplatform::constants::status::*;
use nexum_apdu_macros::apdu_pair;

use super::CLA_GP;

apdu_pair! {
    /// DERIVE KEY command for Keycard
    pub struct DeriveKey {
        command {
            cla: CLA_GP,
            ins: 0xD1,
            required_security_level: SecurityLevel::mac_protected(),
        }

        response {
            ok {
                /// Success response
                #[sw(SW_NO_ERROR)]
                Success
            }

            errors {
                /// Conditions not satisfied
                #[sw(SW_CONDITIONS_NOT_SATISFIED)]
                #[error("Conditions not satisfied")]
                ConditionsNotSatisfied,

                /// Wrong P1/P2: Attempted to derive key less than Invalid derivation sequence
                #[sw(SW_WRONG_P1P2)]
                #[error("Wrong P1/P2: Invalid derivation sequence")]
                WrongP1P2,

                /// Error response
                #[sw(SW_WRONG_DATA)]
                #[error("Wrong data: Derivation sequence is invalid")]
                WrongData,
            }
        }
    }
}
