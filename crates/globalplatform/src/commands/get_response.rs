//! GET RESPONSE command for GlobalPlatform
//!
//! This command is used to retrieve response data when the previous
//! command indicated that more data is available (SW1=61).

use nexum_apdu_macros::apdu_pair;

use crate::constants::*;

apdu_pair! {
    /// GET RESPONSE command for GlobalPlatform
    pub struct GetResponse {
        command {
            cla: cla::ISO7816,
            ins: ins::GET_RESPONSE,

            builders {
                /// Create a GET RESPONSE command with expected length
                pub const fn with_length(length: u8) -> Self {
                    Self::new(0x00, 0x00).with_le(length as ExpectedLength)
                }
            }
        }

        response {
            ok {
                /// Success response
                #[sw(SW_NO_ERROR)]
                #[payload(field = "data")]
                Success {
                    data: Vec<u8>,
                },

                /// More data available (61xx)
                #[sw(0x61, _)]
                #[payload(field = "data")]
                MoreData {
                    sw2: u8, // remaining
                    data: Vec<u8>,
                },
            }

            errors {
                /// Wrong length
                #[sw(SW_WRONG_LENGTH)]
                #[error("Wrong length")]
                WrongLength,
            }

        }
    }
}

impl GetResponseOk {
    /// Get the response data
    pub const fn data(&self) -> &Vec<u8> {
        match self {
            Self::Success { data } | Self::MoreData { data, .. } => data,
        }
    }

    /// Check if more data is available
    pub const fn has_more_data(&self) -> bool {
        matches!(self, Self::MoreData { .. })
    }

    /// Get the number of remaining bytes if more data is available
    pub const fn remaining_bytes(&self) -> Option<u8> {
        match self {
            Self::MoreData { sw2, .. } => Some(*sw2),
            _ => None,
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
    fn test_get_response_command() {
        let cmd = GetResponseCommand::with_length(0xFF);

        assert_eq!(cmd.class(), cla::ISO7816);
        assert_eq!(cmd.instruction(), ins::GET_RESPONSE);
        assert_eq!(cmd.p1(), 0x00);
        assert_eq!(cmd.p2(), 0x00);
        assert_eq!(cmd.data(), None);
        assert_eq!(cmd.expected_length(), Some(0xFF));

        // Test command serialization
        let raw = cmd.to_bytes();
        assert_eq!(raw.as_ref(), hex!("00C00000FF"));
    }

    #[test]
    fn test_get_response_response() {
        // Test successful response
        let response_data = Bytes::from_static(&hex!("010203049000"));
        let result = GetResponseResult::from_bytes(&response_data)
            .unwrap()
            .into_inner()
            .unwrap();

        assert!(matches!(result, GetResponseOk::Success { .. }));
        assert_eq!(result.data(), &hex!("01020304")[..].to_vec());
        assert!(!result.has_more_data());
        assert_eq!(result.remaining_bytes(), None);

        // Test more data available
        let response_data = Bytes::from_static(&hex!("0102030461FF"));
        let result = GetResponseResult::from_bytes(&response_data)
            .unwrap()
            .into_inner()
            .unwrap();

        assert!(matches!(result, GetResponseOk::MoreData { .. }));
        assert_eq!(result.data(), &hex!("01020304")[..].to_vec());
        assert!(result.has_more_data());
        assert_eq!(result.remaining_bytes(), Some(0xFF));
    }
}
