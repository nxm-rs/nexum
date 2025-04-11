//! PUT KEY command for GlobalPlatform
//!
//! This command is used to load or replace keys on the card.

use nexum_apdu_macros::apdu_pair;

use crate::constants::*;

apdu_pair! {
    /// PUT KEY command for GlobalPlatform
    pub struct PutKey {
        command {
            cla: cla::GP,
            ins: ins::PUT_KEY,
            required_security_level: SecurityLevel::mac_protected(),

            builders {
                /// Create a PUT KEY command for loading a new key version
                pub fn new_key_version(key_version: u8, key_data: impl Into<bytes::Bytes>) -> Self {
                    Self::new(0x00, key_version).with_data(key_data.into())
                }

                /// Create a PUT KEY command for replacing an existing key
                pub fn replace_key(key_version: u8, key_data: impl Into<bytes::Bytes>) -> Self {
                    Self::new(0x00, key_version).with_data(key_data.into())
                }

                /// Create a PUT KEY command with key derivation data
                pub fn with_derivation_data(key_version: u8, key_data: impl Into<bytes::Bytes>) -> Self {
                    Self::new(0x01, key_version).with_data(key_data.into())
                }

                /// Create a PUT KEY command for loading multiple keys
                pub fn multiple_keys(key_version: u8, key_data: impl Into<bytes::Bytes>) -> Self {
                    Self::new(0x02, key_version).with_data(key_data.into())
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
                /// Referenced data not found
                #[sw(SW_REFERENCED_DATA_NOT_FOUND)]
                #[error("Referenced data not found")]
                ReferencedDataNotFound,

                /// Security status not satisfied
                #[sw(SW_SECURITY_STATUS_NOT_SATISFIED)]
                #[error("Security status not satisfied")]
                SecurityStatusNotSatisfied,

                /// Wrong data
                #[sw(SW_WRONG_DATA)]
                #[error("Wrong data")]
                WrongData,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use hex_literal::hex;
    use nexum_apdu_core::{ApduCommand, ApduResponse};

    #[test]
    fn test_put_key_command() {
        // Sample key data
        let key_data = hex!("4F07A0000001510000");
        let cmd = PutKeyCommand::new_key_version(0x01, key_data.to_vec());

        assert_eq!(cmd.class(), cla::GP);
        assert_eq!(cmd.instruction(), ins::PUT_KEY);
        assert_eq!(cmd.p1(), 0x00);
        assert_eq!(cmd.p2(), 0x01);
        assert_eq!(cmd.data(), Some(key_data.as_ref()));

        // Test command serialization
        let raw = cmd.to_bytes();
        assert_eq!(raw.as_ref(), hex!("80D80001094F07A0000001510000"));
    }

    #[test]
    fn test_put_key_response() {
        // Test successful response
        let response_data = Bytes::from_static(&hex!("9000"));
        let result = PutKeyResult::from_bytes(&response_data)
            .unwrap()
            .into_inner()
            .unwrap();
        assert!(matches!(result, PutKeyOk::Success));

        // Test error response
        let response_data = Bytes::from_static(&hex!("6982"));
        let response = PutKeyResult::from_bytes(&response_data)
            .unwrap()
            .into_inner()
            .unwrap_err();
        assert!(matches!(response, PutKeyError::SecurityStatusNotSatisfied));
    }
}
