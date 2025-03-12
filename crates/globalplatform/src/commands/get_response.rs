//! GET RESPONSE command for GlobalPlatform
//!
//! This command is used to retrieve response data when the previous
//! command indicated that more data is available (SW1=61).

use apdu_macros::apdu_pair;

use bytes::Bytes;

use crate::constants::{cla, ins, status};

apdu_pair! {
    /// GET RESPONSE command for GlobalPlatform
    pub struct GetResponse {
        command {
            cla: cla::ISO7816,
            ins: ins::GET_RESPONSE,
            secure: false,

            builders {
                /// Create a GET RESPONSE command with expected length
                pub fn with_length(length: u8) -> Self {
                    Self::new(0x00, 0x00).with_le(length as u16)
                }
            }
        }

        response {
            variants {
                /// Success response (9000)
                #[sw(status::SUCCESS)]
                Success {
                    data: bytes::Bytes,
                },

                /// More data available (61xx)
                #[sw(0x61, _)]
                MoreData {
                    sw2: u8, // remaining
                    data: bytes::Bytes,
                },

                /// Wrong length (6700)
                #[sw(status::WRONG_LENGTH)]
                WrongLength,

                /// Other error
                #[sw(_, _)]
                OtherError {
                    sw1: u8,
                    sw2: u8,
                }
            }

            parse_payload = |payload: &[u8], _sw: apdu_core::StatusWord, variant: &mut Self| -> Result<(), apdu_core::Error> {
                match variant {
                    Self::Success { data } | Self::MoreData { data, .. } => {
                        *data = Bytes::copy_from_slice(payload);
                    }
                    _ => {}
                }
                Ok(())
            }

            methods {
                /// Get the response data
                pub fn data(&self) -> Option<&[u8]> {
                    match self {
                        Self::Success { data } | Self::MoreData { data, .. } => Some(data),
                        _ => None,
                    }
                }

                /// Check if more data is available
                pub fn has_more_data(&self) -> bool {
                    matches!(self, Self::MoreData { .. })
                }

                /// Get the number of remaining bytes if more data is available
                pub fn remaining_bytes(&self) -> Option<u8> {
                    match self {
                        Self::MoreData { sw2, .. } => Some(*sw2),
                        _ => None,
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apdu_core::ApduCommand;
    use hex_literal::hex;

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
        let response_data = hex!("010203049000");
        let response = GetResponseResponse::from_bytes(&response_data).unwrap();

        assert!(matches!(response, GetResponseResponse::Success { .. }));
        assert_eq!(response.data(), Some(&hex!("01020304")[..]));
        assert!(!response.has_more_data());
        assert_eq!(response.remaining_bytes(), None);

        // Test more data available
        let response_data = hex!("0102030461FF");
        let response = GetResponseResponse::from_bytes(&response_data).unwrap();

        assert!(matches!(response, GetResponseResponse::MoreData { .. }));
        assert_eq!(response.data(), Some(&hex!("01020304")[..]));
        assert!(response.has_more_data());
        assert_eq!(response.remaining_bytes(), Some(0xFF));
    }
}
