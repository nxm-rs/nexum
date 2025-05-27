use crate::crypto::generate_pairing_token;
use bytes::{Bytes, BytesMut};
use nexum_apdu_core::StatusWord;
use nexum_apdu_globalplatform::constants::status::*;
use nexum_apdu_macros::apdu_pair;

use super::CLA_GP;

const BLOCKED: StatusWord = StatusWord::new(0x63, 0xC0);

apdu_pair! {
    /// VERIFY PIN command for Keycard
    pub struct VerifyPin {
        command {
            cla: CLA_GP,
            ins: 0x20,
            required_security_level: SecurityLevel::enc_mac(),

            builders {
                /// Create a VERIFY PIN command with PIN
                pub fn with_pin(pin: &str) -> Self {
                    Self::new(0x00, 0x00).with_data(Bytes::copy_from_slice(pin.as_bytes()))
                }
            }
        }

        response {
            ok {
                /// Success response
                #[sw(SW_NO_ERROR)]
                Success,
            }

            errors {
                /// PIN is blocked
                #[sw(BLOCKED)]
                #[error("PIN is blocked")]
                PinBlocked,

                /// Wrong PIN - determines remaining attempts from SW2
                #[sw(0x63, _)]
                #[error("Wrong PIN, remaining attempts: {sw2}")]
                WrongPin {
                    /// Extract remaining attempts from SW2 (last 4 bits)
                    sw2: u8,
                },

                /// Wrong data
                #[sw(SW_WRONG_DATA)]
                #[error("Wrong data")]
                WrongData,
            }
        }
    }
}

apdu_pair! {
    /// CHANGE PIN command for Keycard
    pub struct ChangePin {
        command {
            cla: CLA_GP,
            ins: 0x21,
            required_security_level: SecurityLevel::full(),

            builders {
                /// Create a CHANGE PIN command
                pub fn with_pin(pin: &str) -> Self {
                    Self::new(0x00, 0x00).with_data(Bytes::copy_from_slice(pin.as_bytes()))
                }

                /// Create a CHANGE PUK command
                pub fn with_puk(puk: &str) -> Self {
                    Self::new(0x01, 0x00).with_data(Bytes::copy_from_slice(puk.as_bytes()))
                }

                /// Create a CHANGE PAIRING SECRET command
                pub fn with_pairing_secret(secret: &str) -> Self {
                    Self::new(0x02, 0x00).with_data(generate_pairing_token(secret).to_vec())
                }
            }
        }

        response {
            ok {
                /// Success response
                #[sw(SW_NO_ERROR)]
                Success,
            }

            errors {
                /// Security status not satisfied: Must have secure channel open and verified PIN
                #[sw(SW_SECURITY_STATUS_NOT_SATISFIED)]
                #[error("Security status not satisfied: Must have secure channel open and verified PIN")]
                SecurityStatusNotSatisfied,

                /// Incorrect P1/P2: specified the incorrect PIN/PUK/Pairing code to be changed
                #[sw(SW_INCORRECT_P1P2)]
                #[error("Incorrect P1/P2: specified the incorrect PIN/PUK/Pairing code to be changed")]
                IncorrectP1P2,

                /// Wrong data
                #[sw(SW_WRONG_DATA)]
                #[error("Wrong data")]
                WrongData,
            }
        }
    }
}

apdu_pair! {
    /// UNBLOCK PIN command for Keycard
    pub struct UnblockPin {
        command {
            cla: CLA_GP,
            ins: 0x22,
            required_security_level: SecurityLevel::enc_mac(),

            builders {
                /// Create an UNBLOCK PIN command
                pub fn with_puk_and_new_pin(puk: &str, new_pin: &str) -> Self {
                    let mut buf = BytesMut::with_capacity(18);
                    buf.extend(puk.as_bytes());
                    buf.extend(new_pin.as_bytes());
                    Self::new(0x00, 0x00).with_data(buf.freeze())
                }
            }
        }

        response {
            ok {
                /// Success response
                #[sw(SW_NO_ERROR)]
                Success,
            }

            errors {
                /// Security status not satisfied: Must have secure channel open
                #[sw(SW_SECURITY_STATUS_NOT_SATISFIED)]
                #[error("Security status not satisfied: Must have secure channel open")]
                SecurityStatusNotSatisfied,

                /// Conditions not satisfied: PIN must be blocked
                #[sw(SW_CONDITIONS_NOT_SATISFIED)]
                #[error("Conditions not satisfied: PIN must be blocked")]
                ConditionsNotSatisfied,

                /// PUK is blocked
                #[sw(BLOCKED)]
                #[error("PUK is blocked")]
                PukBlocked,

                /// Wrong PUK - determines remaining attempts from SW2
                #[sw(0x63, _)]
                #[error("Wrong PUK, remaining attempts: {sw2}")]
                WrongPuk {
                    /// Extract remaining attempts from SW2 (last 4 bits)
                    sw2: u8,
                },

                /// Wrong data
                #[sw(SW_WRONG_DATA)]
                #[error("Wrong data")]
                WrongData,
            }
        }
    }
}
