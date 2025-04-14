use nexum_apdu_globalplatform::constants::status;
use nexum_apdu_macros::apdu_pair;

use coins_bip32::path::DerivationPath;

use super::CLA_GP;

apdu_pair! {
    /// SET PINLESS PATH command for Keycard
    pub struct SetPinlessPath {
        command {
            cla: CLA_GP,
            ins: 0xC1,
            required_security_level: SecurityLevel::encrypted(),

            builders {
                /// Create a SET PINLESS PATH command with the nominated path
                pub fn with_path(path: &DerivationPath) -> Self {
                    let path_data = path.iter().flat_map(|&x| x.to_be_bytes()).collect::<Vec<_>>();
                    Self::new(0x00, 0x00).with_data(path_data)
                }
            }
        }

        response {
            ok {
                /// Success response
                #[sw(status::SW_NO_ERROR)]
                Success
            }

            errors {
                /// Conditions not satisfied (e.g. secure channel + verified pin)
                #[sw(status::SW_CONDITIONS_NOT_SATISFIED)]
                #[error("Conditions not satisfied: Require secure channel and verified pin")]
                ConditionsNotSatisfied,

                /// Data is not a multiple of 32 bytes
                #[sw(status::SW_WRONG_DATA)]
                #[error("Wrong data: Data must be a multiple of 32 bytes")]
                WrongData,
            }
        }
    }
}
