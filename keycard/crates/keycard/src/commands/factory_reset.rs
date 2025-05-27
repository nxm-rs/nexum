use nexum_apdu_globalplatform::constants::status::*;
use nexum_apdu_macros::apdu_pair;

use super::CLA_GP;

apdu_pair! {
    /// FACTORY RESET command for Keycard
    pub struct FactoryReset {
        command {
            cla: CLA_GP,
            ins: 0xFD,

            builders {
                /// Create a FACTORY RESET command. This is irreversible and requires no authentication.
                pub fn reset() -> Self {
                    Self::new(0xAA, 0x55)
                }
            }
        }

        response {
            ok {
                /// Success response
                #[sw(SW_NO_ERROR)]
                Success
            }
        }
    }
}
